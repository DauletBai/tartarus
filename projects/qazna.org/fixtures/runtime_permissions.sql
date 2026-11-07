-- Run as migration owner in a dedicated Qazna database, after migrations.
-- This group has no login and does not own objects. Provision login secrets separately.
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM pg_roles WHERE rolname='qazna_runtime') THEN
        CREATE ROLE qazna_runtime NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;
    END IF;
    IF EXISTS(SELECT 1 FROM pg_roles WHERE rolname='qazna_runtime' AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls)) THEN
        RAISE EXCEPTION 'unsafe existing qazna_runtime role';
    END IF;
    IF EXISTS(SELECT 1 FROM pg_class WHERE relnamespace='public'::regnamespace AND relowner='qazna_runtime'::regrole) THEN
        RAISE EXCEPTION 'runtime must not own schema objects';
    END IF;
END $$;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO qazna_runtime;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA public TO qazna_runtime;
-- UPDATE permits row locks. Immutable-history triggers reject history changes.
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO qazna_runtime;
REVOKE ALL ON _sqlx_migrations FROM qazna_runtime;
REVOKE INSERT, UPDATE ON evidence_principal_revocations, evidence_enforcement FROM qazna_runtime;
-- No ownership, DELETE, TRUNCATE, trigger modification or schema creation granted.
