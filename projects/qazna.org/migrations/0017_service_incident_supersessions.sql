-- Append-only corrections/withdrawals and many-to-one combinations. Original
-- declarations and previously approved plan links remain historical evidence.
CREATE TABLE service_incident_supersessions (
    incident_id uuid PRIMARY KEY,
    model_id uuid NOT NULL,
    model_generation bigint NOT NULL CHECK (model_generation>0),
    replacement_id uuid,
    kind text NOT NULL CHECK (kind IN ('replaced','merged','withdrawn')),
    basis text NOT NULL CHECK (length(btrim(basis)) BETWEEN 1 AND 1000),
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY(incident_id,model_id) REFERENCES service_incidents(id,model_id),
    FOREIGN KEY(replacement_id,model_id) REFERENCES service_incidents(id,model_id),
    FOREIGN KEY(model_id,model_generation) REFERENCES model_scenario_outcomes(scenario_id,generation),
    CHECK ((kind='withdrawn')=(replacement_id IS NULL)),
    CHECK (replacement_id IS NULL OR replacement_id<>incident_id)
);
CREATE INDEX service_incident_replacement ON service_incident_supersessions(replacement_id);
CREATE TRIGGER immutable_history BEFORE UPDATE OR DELETE OR TRUNCATE ON service_incident_supersessions
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE FUNCTION check_service_incident_supersession() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE original record; replacement record;
BEGIN
    IF NEW.created_xid<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'incident disposition must use its current transaction ID' USING ERRCODE='23514';
    END IF;
    PERFORM 1 FROM model_scenarios WHERE id=NEW.model_id FOR UPDATE;
    IF NEW.model_generation IS DISTINCT FROM (SELECT max(generation) FROM model_scenario_outcomes WHERE scenario_id=NEW.model_id) THEN
        RAISE EXCEPTION 'stale incident correction generation' USING ERRCODE='23514';
    END IF;
    SELECT * INTO STRICT original FROM service_incidents WHERE id=NEW.incident_id AND model_id=NEW.model_id;
    IF EXISTS(SELECT 1 FROM service_incident_supersessions WHERE incident_id=NEW.incident_id) THEN
        RAISE EXCEPTION 'incident already superseded or withdrawn' USING ERRCODE='23514';
    END IF;
    IF NEW.replacement_id IS NOT NULL THEN
        SELECT * INTO STRICT replacement FROM service_incidents WHERE id=NEW.replacement_id AND model_id=NEW.model_id;
        IF replacement.sequence<=original.sequence OR replacement.base_generation<>NEW.model_generation
           OR EXISTS(SELECT 1 FROM service_incident_supersessions WHERE incident_id=NEW.replacement_id)
           OR EXISTS(SELECT 1 FROM service_incident_plans WHERE incident_id=NEW.replacement_id)
           OR EXISTS(SELECT 1 FROM service_incident_supersessions WHERE replacement_id=NEW.replacement_id AND (kind<>'merged' OR NEW.kind<>'merged' OR created_xid<>pg_current_xact_id())) THEN
            RAISE EXCEPTION 'replacement must be a newer active declaration at the current base' USING ERRCODE='23514';
        END IF;
        IF NEW.kind='merged' AND (original.base_generation<>NEW.model_generation
           OR EXISTS(SELECT 1 FROM service_incident_plans WHERE incident_id=NEW.incident_id)
           OR (original.revision->'boundary') IS DISTINCT FROM (replacement.revision->'boundary')
           OR (replacement.revision->>'source_reference') IS DISTINCT FROM ('qazna:incident-combination:'||NEW.replacement_id::text)
           OR NOT ((replacement.revision->'changes') @> (original.revision->'changes'))) THEN
            RAISE EXCEPTION 'combination loses or changes declared evidence' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_service_incident_supersession BEFORE INSERT ON service_incident_supersessions
    FOR EACH ROW EXECUTE FUNCTION check_service_incident_supersession();
-- Whole combination must be committed atomically; no omitted/extra changed cell.
CREATE FUNCTION check_service_incident_combination() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE n bigint; cells bigint; combined jsonb; changes jsonb;
BEGIN
    IF NEW.kind='merged' THEN
        SELECT count(*) INTO n FROM service_incident_supersessions WHERE replacement_id=NEW.replacement_id;
        SELECT revision->'changes' INTO combined FROM service_incidents WHERE id=NEW.replacement_id;
        SELECT jsonb_agg(DISTINCT cell) INTO changes FROM service_incident_supersessions s
            JOIN service_incidents i ON i.id=s.incident_id
            CROSS JOIN LATERAL jsonb_array_elements(i.revision->'changes') cell
            WHERE s.replacement_id=NEW.replacement_id;
        SELECT count(DISTINCT (cell->>'pool',cell->>'period')) INTO cells FROM jsonb_array_elements(changes) cell;
        IF n<2 OR n>16 OR cells>128 OR cells<>jsonb_array_length(changes)
           OR NOT(combined @> changes AND changes @> combined) THEN
            RAISE EXCEPTION 'incomplete or inconsistent incident combination' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE CONSTRAINT TRIGGER complete_service_incident_combination AFTER INSERT ON service_incident_supersessions
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_service_incident_combination();
CREATE FUNCTION check_active_service_incident_plan() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- Preserve the established coupled -> model lock order.
    PERFORM 1 FROM coupled_scenarios WHERE id=NEW.coupled_id FOR UPDATE;
    PERFORM 1 FROM model_scenarios WHERE id=NEW.model_id FOR UPDATE;
    IF EXISTS(SELECT 1 FROM service_incident_supersessions WHERE incident_id=NEW.incident_id) THEN
        RAISE EXCEPTION 'cannot link a superseded or withdrawn incident' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER active_service_incident_plan BEFORE INSERT ON service_incident_plans
    FOR EACH ROW EXECUTE FUNCTION check_active_service_incident_plan();
