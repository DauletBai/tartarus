CREATE TABLE need_norms (
    id uuid PRIMARY KEY,
    code text NOT NULL CHECK (length(code) BETWEEN 1 AND 64),
    revision integer NOT NULL CHECK (revision > 0),
    supersedes uuid UNIQUE REFERENCES need_norms(id),
    category text NOT NULL CHECK (category IN ('food','water','energy')),
    unit text NOT NULL CHECK (length(unit) BETWEEN 1 AND 40),
    quantity bigint NOT NULL CHECK (quantity > 0),
    applicability text NOT NULL CHECK (length(applicability) BETWEEN 1 AND 1000),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    UNIQUE(code,revision),
    CHECK ((revision=1) = (supersedes IS NULL))
);
CREATE TRIGGER immutable_need_norms BEFORE UPDATE OR DELETE OR TRUNCATE ON need_norms
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE FUNCTION check_need_norm_revision() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous record;
BEGIN
    IF NEW.supersedes IS NOT NULL THEN
        SELECT * INTO previous FROM need_norms WHERE id=NEW.supersedes;
        IF NOT FOUND THEN RAISE EXCEPTION 'previous norm must already exist' USING ERRCODE='23503'; END IF;
        IF previous.code<>NEW.code OR previous.revision::bigint+1<>NEW.revision
           OR previous.category<>NEW.category OR previous.unit<>NEW.unit THEN
            RAISE EXCEPTION 'norm revision must continue the same dimension and code' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_norm_revision BEFORE INSERT ON need_norms
    FOR EACH ROW EXECUTE FUNCTION check_need_norm_revision();
ALTER TABLE needs ADD COLUMN created_xid xid8 NOT NULL DEFAULT pg_current_xact_id();
CREATE TABLE need_norm_sources (
    need_id uuid PRIMARY KEY REFERENCES needs(id),
    norm_id uuid NOT NULL REFERENCES need_norms(id)
);
CREATE TRIGGER immutable_need_norm_sources BEFORE UPDATE OR DELETE OR TRUNCATE ON need_norm_sources
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE FUNCTION check_need_norm_source() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE n record; specification record;
BEGIN
    SELECT * INTO n FROM needs WHERE id=NEW.need_id;
    SELECT * INTO specification FROM need_norms WHERE id=NEW.norm_id;
    IF n.created_xid<>pg_current_xact_id() OR n.quantity<>specification.quantity
       OR n.category<>specification.category OR n.unit<>specification.unit THEN
        RAISE EXCEPTION 'norm must match a need created in this transaction' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER valid_need_norm_source AFTER INSERT ON need_norm_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_need_norm_source();
