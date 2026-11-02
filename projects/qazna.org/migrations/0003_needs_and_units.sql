CREATE TABLE provisioning_units (
    id uuid PRIMARY KEY,
    parent_id uuid REFERENCES provisioning_units(id),
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    function text NOT NULL CHECK (length(function) BETWEEN 1 AND 500),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    CHECK (parent_id IS NULL OR parent_id <> id)
);
CREATE INDEX units_by_parent ON provisioning_units(parent_id);
CREATE TRIGGER immutable_units BEFORE UPDATE OR DELETE OR TRUNCATE ON provisioning_units
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

CREATE FUNCTION check_unit_parent() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE depth integer;
BEGIN
    IF NEW.parent_id IS NOT NULL THEN
        IF NOT EXISTS (SELECT 1 FROM provisioning_units WHERE id = NEW.parent_id) THEN
            RAISE EXCEPTION 'parent must already exist' USING ERRCODE = '23503';
        END IF;
        WITH RECURSIVE ancestors AS (
            SELECT id,parent_id,1 AS d FROM provisioning_units WHERE id = NEW.parent_id
            UNION ALL SELECT u.id,u.parent_id,a.d+1 FROM provisioning_units u JOIN ancestors a ON u.id=a.parent_id
        ) SELECT max(d) INTO depth FROM ancestors;
        IF depth >= 64 THEN
            RAISE EXCEPTION 'hierarchy exceeds 64 levels' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_unit_parent BEFORE INSERT ON provisioning_units
    FOR EACH ROW EXECUTE FUNCTION check_unit_parent();

CREATE TABLE unit_providers (
    producer_id uuid PRIMARY KEY REFERENCES accounts(id),
    unit_id uuid NOT NULL REFERENCES provisioning_units(id),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000)
);
CREATE INDEX providers_by_unit ON unit_providers(unit_id);
CREATE TRIGGER immutable_unit_providers BEFORE UPDATE OR DELETE OR TRUNCATE ON unit_providers
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE FUNCTION check_provider_kind() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (SELECT kind FROM accounts WHERE id=NEW.producer_id) <> 'producer' THEN
        RAISE EXCEPTION 'unit provider must have producer account' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_provider BEFORE INSERT ON unit_providers
    FOR EACH ROW EXECUTE FUNCTION check_provider_kind();

CREATE TABLE needs (
    id uuid PRIMARY KEY,
    resident_id uuid NOT NULL REFERENCES accounts(id),
    unit_id uuid NOT NULL REFERENCES provisioning_units(id),
    period date NOT NULL,
    category text NOT NULL CHECK (category IN ('food','water','energy')),
    unit text NOT NULL CHECK (length(unit) BETWEEN 1 AND 40),
    quantity bigint NOT NULL CHECK (quantity > 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    UNIQUE (resident_id,period,category,unit)
);
CREATE INDEX needs_by_unit_period ON needs(unit_id,period);
CREATE TRIGGER immutable_needs BEFORE UPDATE OR DELETE OR TRUNCATE ON needs
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE FUNCTION check_need_resident() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (SELECT kind FROM accounts WHERE id=NEW.resident_id) <> 'resident' THEN
        RAISE EXCEPTION 'need must belong to resident' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER valid_need BEFORE INSERT ON needs
    FOR EACH ROW EXECUTE FUNCTION check_need_resident();

-- Existing reservations remain unattributed. Only reservations created together
-- with a successful needs attempt can contribute to that need's coverage.
ALTER TABLE reservations ADD COLUMN created_xid xid8 NOT NULL DEFAULT pg_current_xact_id();
CREATE TABLE need_attempts (
    id uuid PRIMARY KEY,
    need_id uuid NOT NULL REFERENCES needs(id),
    resource_id uuid NOT NULL REFERENCES resources(id),
    quantity bigint NOT NULL CHECK (quantity > 0),
    reservation_id uuid UNIQUE REFERENCES reservations(id),
    reasons text[] NOT NULL,
    uncovered_before bigint NOT NULL CHECK (uncovered_before >= 0),
    resource_available bigint NOT NULL CHECK (resource_available >= 0),
    accounting_available bigint NOT NULL CHECK (accounting_available >= 0),
    required_measure bigint CHECK (required_measure > 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK ((reservation_id IS NOT NULL) = (cardinality(reasons) = 0)),
    CHECK (array_position(reasons,NULL) IS NULL),
    CHECK (reasons <@ ARRAY['coverage_limit','physical_shortage','accounting_shortage','quantity_limit']::text[]),
    CHECK (reservation_id IS NULL OR required_measure IS NOT NULL)
);
CREATE INDEX attempts_by_need ON need_attempts(need_id,created_at DESC,id DESC);
CREATE TRIGGER immutable_need_attempts BEFORE UPDATE OR DELETE OR TRUNCATE ON need_attempts
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

CREATE VIEW need_totals AS
SELECT n.*, totals.reserved, totals.received,
       n.quantity - totals.reserved - totals.received AS uncovered,
       n.quantity - totals.received AS unmet,
       last.id AS last_attempt_id, last.reasons AS last_attempt_reasons
FROM needs n
CROSS JOIN LATERAL (
    SELECT COALESCE(sum(a.quantity) FILTER (WHERE o.state IS NULL),0) AS reserved,
           COALESCE(sum(a.quantity) FILTER (WHERE o.state='settled'),0) AS received
    FROM need_attempts a LEFT JOIN reservation_outcomes o ON o.reservation_id=a.reservation_id
    WHERE a.need_id=n.id AND a.reservation_id IS NOT NULL
) totals
LEFT JOIN LATERAL (
    SELECT id,reasons FROM need_attempts WHERE need_id=n.id ORDER BY created_at DESC,id DESC LIMIT 1
) last ON true;

CREATE FUNCTION check_need_attempt() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE n record; r record; s record; covered numeric; provider_in_scope boolean;
BEGIN
    SELECT * INTO n FROM needs WHERE id=NEW.need_id FOR UPDATE;
    IF NEW.reservation_id IS NOT NULL THEN
        SELECT * INTO r FROM reservations WHERE id=NEW.reservation_id;
        SELECT * INTO s FROM resources WHERE id=r.resource_id;
        WITH RECURSIVE scope AS (
            SELECT id FROM provisioning_units WHERE id=n.unit_id
            UNION ALL SELECT u.id FROM provisioning_units u JOIN scope p ON u.parent_id=p.id
        ) SELECT EXISTS (SELECT 1 FROM unit_providers p JOIN scope u ON u.id=p.unit_id WHERE p.producer_id=s.producer_id) INTO provider_in_scope;
        IF r.created_xid <> pg_current_xact_id() OR r.resident_id <> n.resident_id
           OR r.quantity <> NEW.quantity OR r.resource_id <> NEW.resource_id
           OR r.amount <> NEW.required_measure OR s.category <> n.category OR s.unit <> n.unit
           OR NOT provider_in_scope THEN
            RAISE EXCEPTION 'reservation does not match need' USING ERRCODE = '23514';
        END IF;
        SELECT reserved+received INTO covered FROM need_totals WHERE id=n.id;
        IF covered > n.quantity THEN
            RAISE EXCEPTION 'need coverage exceeds requested quantity' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER valid_need_attempt AFTER INSERT ON need_attempts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_need_attempt();
