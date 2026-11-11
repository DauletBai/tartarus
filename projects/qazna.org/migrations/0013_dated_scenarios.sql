-- Research scenario history, deliberately separate from inventory and money.
CREATE TABLE dated_scenarios (
    id uuid PRIMARY KEY,
    definition jsonb NOT NULL CHECK (jsonb_typeof(definition)='object'),
    initial bytea NOT NULL CHECK (octet_length(initial) BETWEEN 1 AND 262144),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE dated_scenario_drafts (
    id uuid PRIMARY KEY,
    scenario_id uuid NOT NULL REFERENCES dated_scenarios(id),
    base_generation bigint NOT NULL CHECK (base_generation>=0),
    request jsonb NOT NULL CHECK (jsonb_typeof(request)='object'),
    candidate bytea NOT NULL CHECK (octet_length(candidate) BETWEEN 1 AND 262144),
    flow_revision bigint NOT NULL CHECK (flow_revision>0),
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(id,scenario_id)
);
CREATE TABLE dated_scenario_outcomes (
    draft_id uuid PRIMARY KEY,
    scenario_id uuid NOT NULL,
    generation bigint CHECK (generation>0),
    state text NOT NULL CHECK (state IN ('applied','discarded')),
    basis text NOT NULL CHECK (length(btrim(basis)) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY(draft_id,scenario_id) REFERENCES dated_scenario_drafts(id,scenario_id),
    UNIQUE(scenario_id,generation),
    CHECK ((state='applied')=(generation IS NOT NULL))
);
DO $$ DECLARE t text; BEGIN
    FOREACH t IN ARRAY ARRAY['dated_scenarios','dated_scenario_drafts','dated_scenario_outcomes'] LOOP
        EXECUTE format('CREATE TRIGGER immutable_history BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change()', t);
    END LOOP;
END $$;
CREATE TRIGGER current_transaction_xid BEFORE INSERT ON dated_scenario_drafts
    FOR EACH ROW EXECUTE FUNCTION check_new_transaction_xid();
CREATE FUNCTION check_dated_scenario_outcome() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE d record; current_generation bigint; local_full_xid numeric; current_full_xid numeric;
BEGIN
    PERFORM 1 FROM dated_scenarios WHERE id=NEW.scenario_id FOR UPDATE;
    SELECT *, xmin AS row_xmin INTO d FROM dated_scenario_drafts WHERE id=NEW.draft_id AND scenario_id=NEW.scenario_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'scenario draft not found' USING ERRCODE='23514';
    END IF;
    -- Historical XIDs can collide after restore. Consult the local inserting
    -- transaction as well, including current savepoint/subtransaction tuples.
    IF d.created_xid=pg_current_xact_id() THEN
        current_full_xid:=pg_current_xact_id()::text::numeric;
        local_full_xid:=current_full_xid-pg_current_xact_id()::xid::text::bigint+d.row_xmin::text::bigint;
        IF local_full_xid-current_full_xid>2147483648 THEN local_full_xid:=local_full_xid-4294967296; END IF;
        IF current_full_xid-local_full_xid>2147483648 THEN local_full_xid:=local_full_xid+4294967296; END IF;
        IF local_full_xid<0 OR pg_xact_status(local_full_xid::text::xid8) IS DISTINCT FROM 'committed' THEN
            RAISE EXCEPTION 'review a previously committed scenario draft' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.state='applied' THEN
        SELECT COALESCE(max(generation),0) INTO current_generation FROM dated_scenario_outcomes WHERE scenario_id=NEW.scenario_id;
        IF d.base_generation<>current_generation OR NEW.generation<>current_generation+1 THEN
            RAISE EXCEPTION 'stale scenario generation' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_dated_scenario_outcome BEFORE INSERT ON dated_scenario_outcomes
    FOR EACH ROW EXECUTE FUNCTION check_dated_scenario_outcome();
