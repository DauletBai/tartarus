-- Revocation stops future use of a principal without rewriting past statements.
CREATE TABLE evidence_principal_revocations (
    principal_id uuid PRIMARY KEY REFERENCES evidence_principals(id),
    basis text NOT NULL CHECK(length(basis) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER immutable_history BEFORE UPDATE OR DELETE OR TRUNCATE ON evidence_principal_revocations
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE FUNCTION lock_revoked_principal() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM 1 FROM evidence_principals WHERE id=NEW.principal_id FOR UPDATE;
    RETURN NEW;
END;
$$;
CREATE TRIGGER lock_revocation BEFORE INSERT ON evidence_principal_revocations
    FOR EACH ROW EXECUTE FUNCTION lock_revoked_principal();
CREATE FUNCTION require_active_principal() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM 1 FROM evidence_principals WHERE id=NEW.principal_id FOR SHARE;
    IF EXISTS(SELECT 1 FROM evidence_principal_revocations WHERE principal_id=NEW.principal_id) THEN
        RAISE EXCEPTION 'principal revoked' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER active_principal BEFORE INSERT ON evidence_statements
    FOR EACH ROW EXECUTE FUNCTION require_active_principal();
