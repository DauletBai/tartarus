CREATE TABLE planning_drafts (
    id uuid PRIMARY KEY,
    format_version integer NOT NULL DEFAULT 1 CHECK (format_version=1),
    request jsonb NOT NULL CHECK (jsonb_typeof(request)='object'),
    proposal jsonb NOT NULL CHECK (jsonb_typeof(proposal)='object' AND jsonb_typeof(proposal->'needs')='array'),
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE planning_outcomes (
    draft_id uuid PRIMARY KEY REFERENCES planning_drafts(id),
    state text NOT NULL CHECK (state IN ('applied','discarded')),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE planning_actions (
    draft_id uuid NOT NULL REFERENCES planning_outcomes(draft_id) DEFERRABLE INITIALLY DEFERRED,
    need_position integer NOT NULL CHECK (need_position BETWEEN 0 AND 31),
    action_position integer NOT NULL CHECK (action_position BETWEEN 0 AND 63),
    kind text NOT NULL CHECK (kind IN ('stock','production')),
    need_id uuid NOT NULL REFERENCES needs(id),
    reservation_id uuid UNIQUE REFERENCES reservations(id),
    attempt_id uuid UNIQUE REFERENCES need_attempts(id),
    production_plan_id uuid UNIQUE REFERENCES production_plans(id),
    PRIMARY KEY(draft_id,need_position,kind,action_position),
    CHECK ((kind='stock' AND reservation_id IS NOT NULL AND attempt_id IS NOT NULL AND production_plan_id IS NULL)
        OR (kind='production' AND reservation_id IS NULL AND attempt_id IS NULL AND production_plan_id IS NOT NULL))
);
DO $$ DECLARE t text; BEGIN
    FOREACH t IN ARRAY ARRAY['planning_drafts','planning_outcomes','planning_actions'] LOOP
        EXECUTE format('CREATE TRIGGER immutable_history BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change()', t);
    END LOOP;
END $$;
CREATE FUNCTION lock_planning_draft() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM 1 FROM planning_drafts WHERE id=NEW.draft_id FOR UPDATE;
    RETURN NEW;
END;
$$;
CREATE TRIGGER lock_planning_outcome BEFORE INSERT ON planning_outcomes FOR EACH ROW EXECUTE FUNCTION lock_planning_draft();
CREATE TRIGGER lock_planning_action BEFORE INSERT ON planning_actions FOR EACH ROW EXECUTE FUNCTION lock_planning_draft();
CREATE FUNCTION check_planning_outcome() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE d record; o record; a record; r record; p record; n record; recipe record;
        line jsonb; expected jsonb; expected_count bigint; in_scope boolean;
BEGIN
    SELECT * INTO d FROM planning_drafts WHERE id=NEW.draft_id;
    SELECT * INTO o FROM planning_outcomes WHERE draft_id=NEW.draft_id;
    IF NOT FOUND THEN RAISE EXCEPTION 'draft action requires terminal outcome' USING ERRCODE='23514'; END IF;
    IF o.created_xid<>pg_current_xact_id() OR o.created_xid=d.created_xid THEN
        RAISE EXCEPTION 'close a previously committed draft; actions must share its closing transaction' USING ERRCODE='23514';
    END IF;
    IF o.state='discarded' THEN
        IF EXISTS(SELECT 1 FROM planning_actions WHERE draft_id=d.id) THEN
            RAISE EXCEPTION 'discarded draft cannot have actions' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
    SELECT COALESCE(sum(jsonb_array_length(x->'stock')+jsonb_array_length(x->'production')),0)
        INTO expected_count FROM jsonb_array_elements(d.proposal->'needs') x;
    IF expected_count=0 OR expected_count<>(SELECT count(*) FROM planning_actions WHERE draft_id=d.id) THEN
        RAISE EXCEPTION 'applied draft must have exactly all reviewed actions' USING ERRCODE='23514';
    END IF;
    FOR a IN SELECT * FROM planning_actions WHERE draft_id=d.id LOOP
        line:=d.proposal->'needs'->a.need_position;
        expected:=line->a.kind->a.action_position;
        IF expected IS NULL OR (line->>'need')::uuid<>a.need_id THEN
            RAISE EXCEPTION 'action does not match draft position' USING ERRCODE='23514';
        END IF;
        IF a.kind='stock' THEN
            SELECT * INTO r FROM reservations WHERE id=a.reservation_id;
            IF r.created_xid<>o.created_xid OR r.resource_id<>(expected->>'resource')::uuid
               OR r.quantity<>(expected->>'quantity')::bigint OR r.amount<>(expected->>'required_measure')::bigint
               OR NOT EXISTS(SELECT 1 FROM need_attempts WHERE id=a.attempt_id AND need_id=a.need_id AND reservation_id=r.id) THEN
                RAISE EXCEPTION 'reviewed stock reservation mismatch' USING ERRCODE='23514';
            END IF;
        ELSE
            SELECT * INTO p FROM production_plans WHERE id=a.production_plan_id;
            IF p.created_xid<>o.created_xid OR p.recipe_id<>(expected#>>'{request,recipe}')::uuid
               OR p.capacity_id<>(expected#>>'{request,capacity}')::uuid OR p.batches<>(expected#>>'{request,batches}')::bigint
               OR p.basis<>(expected#>>'{request,basis}')
               OR (SELECT count(*) FROM production_plan_inputs WHERE plan_id=p.id)<>jsonb_array_length(expected#>'{request,inputs}')
               OR EXISTS(SELECT 1 FROM jsonb_array_elements(expected#>'{request,inputs}') i WHERE NOT EXISTS
                  (SELECT 1 FROM production_plan_inputs WHERE plan_id=p.id AND slot=i->>'slot' AND resource_id=(i->>'resource')::uuid)) THEN
                RAISE EXCEPTION 'reviewed production plan mismatch' USING ERRCODE='23514';
            END IF;
            SELECT * INTO n FROM needs WHERE id=a.need_id;
            SELECT * INTO recipe FROM production_recipes WHERE id=p.recipe_id;
            WITH RECURSIVE scope AS (SELECT id FROM provisioning_units WHERE id=n.unit_id
                UNION ALL SELECT u.id FROM provisioning_units u JOIN scope parent ON u.parent_id=parent.id)
                SELECT EXISTS(SELECT 1 FROM scope WHERE id=recipe.unit_id) INTO in_scope;
            IF NOT in_scope OR recipe.output_category<>n.category OR recipe.output_unit<>n.unit
               OR (SELECT period FROM production_capacities WHERE id=p.capacity_id)<>n.period THEN
                RAISE EXCEPTION 'production does not serve the reviewed need scope/dimension/period' USING ERRCODE='23514';
            END IF;
        END IF;
    END LOOP;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER valid_planning_outcome AFTER INSERT ON planning_outcomes
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_planning_outcome();
CREATE CONSTRAINT TRIGGER valid_planning_action AFTER INSERT ON planning_actions
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_planning_outcome();
