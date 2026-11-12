-- Observations remain durable even when no feasible repair exists. A linked
-- reviewed plan is not a claim that a real-world incident has been resolved.
CREATE TABLE service_incidents (
    id uuid PRIMARY KEY,
    sequence bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    model_id uuid NOT NULL,
    base_generation bigint NOT NULL CHECK (base_generation>0),
    revision jsonb NOT NULL CHECK (jsonb_typeof(revision)='object' AND octet_length(revision::text)<=32768),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY(model_id,base_generation) REFERENCES model_scenario_outcomes(scenario_id,generation),
    UNIQUE(id,model_id)
);
CREATE INDEX service_incidents_by_model ON service_incidents(model_id,sequence);
CREATE TABLE service_incident_plans (
    incident_id uuid PRIMARY KEY,
    model_id uuid NOT NULL,
    model_generation bigint NOT NULL CHECK (model_generation>0),
    coupled_id uuid NOT NULL,
    coupled_generation bigint NOT NULL CHECK (coupled_generation>0),
    basis text NOT NULL CHECK (length(btrim(basis)) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY(incident_id,model_id) REFERENCES service_incidents(id,model_id),
    FOREIGN KEY(model_id,model_generation) REFERENCES model_scenario_outcomes(scenario_id,generation),
    FOREIGN KEY(coupled_id,coupled_generation) REFERENCES coupled_scenario_outcomes(scenario_id,generation),
    FOREIGN KEY(coupled_id,model_id) REFERENCES coupled_scenarios(id,model_id)
);
DO $$ DECLARE t text; BEGIN
    FOREACH t IN ARRAY ARRAY['service_incidents','service_incident_plans'] LOOP
        EXECUTE format('CREATE TRIGGER immutable_history BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change()', t);
    END LOOP;
END $$;
CREATE FUNCTION check_service_incident_plan() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE incident record; old jsonb; physical jsonb; money jsonb; revisions jsonb; n integer;
BEGIN
    -- Same lock order as coupled scenario review.
    PERFORM 1 FROM coupled_scenarios WHERE id=NEW.coupled_id FOR UPDATE;
    PERFORM 1 FROM model_scenarios WHERE id=NEW.model_id FOR UPDATE;
    SELECT * INTO incident FROM service_incidents WHERE id=NEW.incident_id AND model_id=NEW.model_id;
    IF NOT FOUND OR NEW.model_generation<=incident.base_generation
       OR NEW.model_generation IS DISTINCT FROM (SELECT max(generation) FROM model_scenario_outcomes WHERE scenario_id=NEW.model_id)
       OR NEW.coupled_generation IS DISTINCT FROM (SELECT max(generation) FROM coupled_scenario_outcomes WHERE scenario_id=NEW.coupled_id) THEN
        RAISE EXCEPTION 'incident plan requires current approved generations' USING ERRCODE='23514';
    END IF;
    SELECT convert_from(d.candidate,'UTF8')::jsonb INTO old FROM model_scenario_outcomes o JOIN model_scenario_drafts d ON d.id=o.draft_id WHERE o.scenario_id=NEW.model_id AND o.generation=incident.base_generation;
    SELECT convert_from(d.candidate,'UTF8')::jsonb INTO physical FROM model_scenario_outcomes o JOIN model_scenario_drafts d ON d.id=o.draft_id WHERE o.scenario_id=NEW.model_id AND o.generation=NEW.model_generation;
    SELECT convert_from(d.candidate,'UTF8')::jsonb INTO money FROM coupled_scenario_outcomes o JOIN coupled_scenario_drafts d ON d.id=o.draft_id WHERE o.scenario_id=NEW.coupled_id AND o.generation=NEW.coupled_generation;
    revisions:=COALESCE(old#>'{input,services,revisions}','[]'::jsonb);
    n:=jsonb_array_length(revisions);
    IF physical#>ARRAY['input','services','revisions',n::text] IS DISTINCT FROM incident.revision
       OR EXISTS (SELECT 1 FROM jsonb_array_elements(revisions) WITH ORDINALITY r(value,i) WHERE physical#>ARRAY['input','services','revisions',(i-1)::text] IS DISTINCT FROM r.value)
       OR money#>>'{evidence,physical,definition,id}' IS DISTINCT FROM NEW.model_id::text
       OR money#>>'{evidence,physical,generation}' IS DISTINCT FROM NEW.model_generation::text
       OR money#>'{evidence,all_scheduled_payments_settled}' IS DISTINCT FROM 'true'::jsonb
       OR money#>'{evidence,failed}' IS DISTINCT FROM '[]'::jsonb THEN
        RAISE EXCEPTION 'incident plan evidence mismatch' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_service_incident_plan BEFORE INSERT ON service_incident_plans
    FOR EACH ROW EXECUTE FUNCTION check_service_incident_plan();
