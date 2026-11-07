-- Durable coordination metadata never authorizes economic actions.
CREATE TABLE coordination_monitors (
    id uuid PRIMARY KEY,
    request jsonb NOT NULL CHECK (jsonb_typeof(request)='object'),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE coordination_stops (
    monitor_id uuid PRIMARY KEY REFERENCES coordination_monitors(id),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
-- Events and jobs are mutable delivery state. Economic history remains immutable.
CREATE TABLE coordination_events (
    command_key text PRIMARY KEY REFERENCES commands(key),
    dispatched boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX coordination_events_pending ON coordination_events(created_at) WHERE NOT dispatched;
CREATE TABLE coordination_jobs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    monitor_id uuid NOT NULL REFERENCES coordination_monitors(id),
    command_key text REFERENCES coordination_events(command_key),
    state text NOT NULL DEFAULT 'queued' CHECK (state IN ('queued','running','completed','failed','cancelled')),
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 3),
    lease_token uuid,
    lease_until timestamptz,
    available_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    error_code text,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE NULLS NOT DISTINCT(monitor_id,command_key),
    CHECK ((state='running')=(lease_token IS NOT NULL AND lease_until IS NOT NULL))
);
CREATE INDEX coordination_jobs_pending ON coordination_jobs(monitor_id,created_at) WHERE state IN ('queued','running');
CREATE TABLE coordination_results (
    sequence bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    job_id uuid PRIMARY KEY REFERENCES coordination_jobs(id),
    engine text NOT NULL CHECK (engine='ordered-planner-v1'),
    proposal jsonb NOT NULL CHECK (jsonb_typeof(proposal)='object'),
    draft_id uuid NOT NULL REFERENCES planning_drafts(id),
    changed boolean NOT NULL,
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE coordination_reviews (
    id uuid PRIMARY KEY,
    job_id uuid NOT NULL REFERENCES coordination_results(job_id),
    provider text NOT NULL CHECK (provider='codex-chatgpt'),
    model text NOT NULL CHECK (length(model) BETWEEN 1 AND 100),
    report jsonb NOT NULL CHECK (jsonb_typeof(report)='object'),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
DO $$ DECLARE t text; BEGIN
    FOREACH t IN ARRAY ARRAY['coordination_monitors','coordination_stops','coordination_results','coordination_reviews'] LOOP
        EXECUTE format('CREATE TRIGGER immutable_history BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change()',t);
    END LOOP;
END $$;
CREATE FUNCTION enqueue_coordination_event() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- No feedback loop from advice, monitor controls or AI commentary.
    IF NOT NEW.request ?| ARRAY['planning_draft','discard_planning_draft','coordination_monitor','coordination_stop','coordination_review'] THEN
        INSERT INTO coordination_events(command_key) VALUES (NEW.key);
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER coordination_command_event AFTER INSERT ON commands
    FOR EACH ROW EXECUTE FUNCTION enqueue_coordination_event();
CREATE FUNCTION check_coordination_result() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE j record; d record; m record;
BEGIN
    SELECT * INTO j FROM coordination_jobs WHERE id=NEW.job_id;
    SELECT * INTO m FROM coordination_monitors WHERE id=j.monitor_id;
    SELECT * INTO d FROM planning_drafts WHERE id=NEW.draft_id;
    IF j.state<>'completed' OR d.request<>m.request
       OR (d.proposal-'observed_at')<>(NEW.proposal-'observed_at')
       OR (NEW.changed AND d.created_xid<>NEW.created_xid) THEN
        RAISE EXCEPTION 'coordination result must match completed job and reviewed draft' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER valid_coordination_result AFTER INSERT ON coordination_results
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_coordination_result();
