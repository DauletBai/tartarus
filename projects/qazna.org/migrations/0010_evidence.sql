-- Explicit opt-in enforcement preserves historical research databases.
-- Activation is permanent and serialized against economic writes.
CREATE TABLE evidence_enforcement (
    singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
    basis text NOT NULL CHECK(length(basis) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE evidence_principals (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id),
    role text NOT NULL CHECK(role IN ('inspector','custodian','recipient')),
    basis text NOT NULL CHECK(length(basis) BETWEEN 1 AND 1000),
    UNIQUE(account_id,role)
);
CREATE TABLE evidence_statements (
    id uuid PRIMARY KEY,
    principal_id uuid NOT NULL REFERENCES evidence_principals(id),
    resource_id uuid REFERENCES resources(id),
    reservation_id uuid REFERENCES reservations(id),
    decision text NOT NULL CHECK(decision IN ('confirm','dispute')),
    quantity bigint NOT NULL CHECK(quantity>0),
    evidence_sha256 text NOT NULL CHECK(evidence_sha256 ~ '^[a-f0-9]{64}$'),
    basis text NOT NULL CHECK(length(basis) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK((resource_id IS NULL) <> (reservation_id IS NULL)),
    UNIQUE NULLS NOT DISTINCT(principal_id,resource_id,reservation_id,decision)
);
DO $$ DECLARE t text; BEGIN
    FOREACH t IN ARRAY ARRAY['evidence_enforcement','evidence_principals','evidence_statements'] LOOP
        EXECUTE format('CREATE TRIGGER immutable_history BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change()',t);
    END LOOP;
END $$;
CREATE FUNCTION resource_attested(target uuid) RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT NOT EXISTS(SELECT 1 FROM evidence_statements WHERE resource_id=target AND decision='dispute')
      AND (SELECT count(DISTINCT p.account_id) FROM evidence_statements s JOIN evidence_principals p ON p.id=s.principal_id
           WHERE s.resource_id=target AND s.decision='confirm' AND p.role='inspector')>=2;
$$;
CREATE FUNCTION delivery_attested(target uuid) RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT NOT EXISTS(SELECT 1 FROM evidence_statements WHERE reservation_id=target AND decision='dispute')
      AND (SELECT count(DISTINCT p.role) FROM evidence_statements s JOIN evidence_principals p ON p.id=s.principal_id
           WHERE s.reservation_id=target AND s.decision='confirm' AND p.role IN ('custodian','recipient'))=2;
$$;
CREATE FUNCTION check_evidence_statement() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p record; s record; r record;
BEGIN
    SELECT * INTO p FROM evidence_principals WHERE id=NEW.principal_id;
    IF NEW.resource_id IS NOT NULL THEN
        SELECT * INTO s FROM resources WHERE id=NEW.resource_id FOR UPDATE;
        IF p.role<>'inspector' OR p.account_id=s.producer_id OR NEW.quantity<>s.initial_quantity THEN
            RAISE EXCEPTION 'resource evidence needs an independent inspector and exact declared quantity' USING ERRCODE='23514';
        END IF;
    ELSE
        SELECT * INTO r FROM reservations WHERE id=NEW.reservation_id FOR UPDATE;
        SELECT * INTO s FROM resources WHERE id=r.resource_id;
        IF NEW.quantity<>r.quantity OR NOT ((p.role='recipient' AND p.account_id=r.resident_id)
             OR (p.role='custodian' AND p.account_id=s.producer_id)) THEN
            RAISE EXCEPTION 'delivery evidence must be from the actual custodian or recipient' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_evidence BEFORE INSERT ON evidence_statements
    FOR EACH ROW EXECUTE FUNCTION check_evidence_statement();
CREATE FUNCTION enforce_resource_evidence() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE target uuid; r record;
BEGIN
    PERFORM pg_advisory_xact_lock_shared(91020261103);
    IF NOT EXISTS(SELECT 1 FROM evidence_enforcement) THEN RETURN NEW; END IF;
    IF TG_TABLE_NAME='reservation_outcomes' THEN
        IF NEW.state<>'settled' THEN RETURN NEW; END IF;
        SELECT * INTO r FROM reservations WHERE id=NEW.reservation_id FOR UPDATE;
        target:=r.resource_id;
        IF NOT delivery_attested(r.id) THEN
            RAISE EXCEPTION 'delivery evidence required' USING ERRCODE='23514';
        END IF;
    ELSE target:=NEW.resource_id;
    END IF;
    PERFORM 1 FROM resources WHERE id=target FOR UPDATE;
    IF NOT resource_attested(target) THEN
        RAISE EXCEPTION 'resource evidence required' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
-- Run after the existing dependency locks to preserve account/resource order.
CREATE TRIGGER z_require_evidence BEFORE INSERT ON reservations
    FOR EACH ROW EXECUTE FUNCTION enforce_resource_evidence();
CREATE TRIGGER z_require_evidence BEFORE INSERT ON reservation_outcomes
    FOR EACH ROW EXECUTE FUNCTION enforce_resource_evidence();
CREATE TRIGGER z_require_evidence BEFORE INSERT ON production_plan_inputs
    FOR EACH ROW EXECUTE FUNCTION enforce_resource_evidence();
CREATE FUNCTION activate_evidence() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(91020261103);
    IF EXISTS(SELECT 1 FROM reservations r WHERE NOT EXISTS(SELECT 1 FROM reservation_outcomes o WHERE o.reservation_id=r.id))
       OR EXISTS(SELECT 1 FROM production_plans p WHERE NOT EXISTS(SELECT 1 FROM production_completions c WHERE c.plan_id=p.id)
                 AND NOT EXISTS(SELECT 1 FROM production_cancellations c WHERE c.plan_id=p.id)) THEN
        RAISE EXCEPTION 'resolve outstanding holds before enabling evidence enforcement' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER activate_evidence BEFORE INSERT ON evidence_enforcement
    FOR EACH ROW EXECUTE FUNCTION activate_evidence();
CREATE FUNCTION check_evidence_principal() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE k text;
BEGIN
    SELECT kind INTO k FROM accounts WHERE id=NEW.account_id;
    IF k='issuer' OR (NEW.role='recipient' AND k<>'resident') OR (NEW.role='custodian' AND k<>'producer') THEN
        RAISE EXCEPTION 'principal account does not match role' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_principal BEFORE INSERT ON evidence_principals FOR EACH ROW EXECUTE FUNCTION check_evidence_principal();
CREATE FUNCTION enforce_completion_evidence() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock_shared(91020261103);
    IF EXISTS(SELECT 1 FROM evidence_enforcement) AND EXISTS(
       SELECT 1 FROM production_plan_inputs WHERE plan_id=NEW.plan_id AND NOT resource_attested(resource_id)) THEN
        RAISE EXCEPTION 'production inputs need undisputed evidence' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER z_require_evidence BEFORE INSERT ON production_completions
    FOR EACH ROW EXECUTE FUNCTION enforce_completion_evidence();
