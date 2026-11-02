ALTER TABLE journal_transactions DROP CONSTRAINT journal_transactions_kind_check;
ALTER TABLE journal_transactions ADD CHECK (kind IN ('transfer','genesis','allocation','settlement'));

CREATE TABLE commands (
    key text PRIMARY KEY CHECK (length(key) BETWEEN 1 AND 128),
    request jsonb NOT NULL,
    response jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER immutable_commands BEFORE UPDATE OR DELETE OR TRUNCATE ON commands
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

CREATE TABLE allocations (
    id uuid PRIMARY KEY,
    resident_id uuid NOT NULL REFERENCES accounts(id),
    period date NOT NULL,
    category text NOT NULL CHECK (category IN ('food','water','energy')),
    amount bigint NOT NULL CHECK (amount > 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    transaction_id uuid NOT NULL UNIQUE REFERENCES journal_transactions(id) DEFERRABLE INITIALLY DEFERRED,
    UNIQUE (resident_id, period, category)
);
CREATE TRIGGER immutable_allocations BEFORE UPDATE OR DELETE OR TRUNCATE ON allocations
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

-- A resource is an explicitly declared physical lot, not a claim of real growth.
CREATE TABLE resources (
    id uuid PRIMARY KEY,
    producer_id uuid NOT NULL REFERENCES accounts(id),
    category text NOT NULL CHECK (category IN ('food','water','energy')),
    unit text NOT NULL CHECK (length(unit) BETWEEN 1 AND 40),
    initial_quantity bigint NOT NULL CHECK (initial_quantity > 0),
    unit_measure bigint NOT NULL CHECK (unit_measure > 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000)
);
CREATE TRIGGER immutable_resources BEFORE UPDATE OR DELETE OR TRUNCATE ON resources
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

CREATE TABLE reservations (
    id uuid PRIMARY KEY,
    resident_id uuid NOT NULL REFERENCES accounts(id),
    resource_id uuid NOT NULL REFERENCES resources(id),
    quantity bigint NOT NULL CHECK (quantity > 0),
    amount bigint NOT NULL CHECK (amount > 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX reservations_by_resident ON reservations(resident_id);
CREATE INDEX reservations_by_resource ON reservations(resource_id);
CREATE TRIGGER immutable_reservations BEFORE UPDATE OR DELETE OR TRUNCATE ON reservations
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

CREATE TABLE reservation_outcomes (
    reservation_id uuid PRIMARY KEY REFERENCES reservations(id),
    state text NOT NULL CHECK (state IN ('settled','cancelled')),
    transaction_id uuid UNIQUE REFERENCES journal_transactions(id) DEFERRABLE INITIALLY DEFERRED,
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK ((state = 'settled') = (transaction_id IS NOT NULL))
);
CREATE TRIGGER immutable_outcomes BEFORE UPDATE OR DELETE OR TRUNCATE ON reservation_outcomes
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

CREATE VIEW account_totals AS
SELECT a.id, a.kind,
    COALESCE((SELECT sum(p.delta) FROM postings p WHERE p.account_id = a.id), 0) AS balance,
    COALESCE((SELECT sum(r.amount) FROM reservations r
        WHERE r.resident_id = a.id AND NOT EXISTS
        (SELECT 1 FROM reservation_outcomes o WHERE o.reservation_id = r.id)), 0) AS reserved
FROM accounts a;

CREATE VIEW resource_totals AS
SELECT s.*,
    COALESCE((SELECT sum(r.quantity) FROM reservations r
        JOIN reservation_outcomes o ON o.reservation_id = r.id AND o.state = 'settled'
        WHERE r.resource_id = s.id), 0) AS consumed,
    COALESCE((SELECT sum(r.quantity) FROM reservations r
        WHERE r.resource_id = s.id AND NOT EXISTS
        (SELECT 1 FROM reservation_outcomes o WHERE o.reservation_id = r.id)), 0) AS reserved
FROM resources s;

CREATE FUNCTION assert_account_funds(target uuid) RETURNS void LANGUAGE plpgsql AS $$
DECLARE a record;
BEGIN
    SELECT * INTO a FROM account_totals WHERE id = target;
    IF a.balance > 9223372036854775807 OR a.balance < -9223372036854775807
       OR (a.kind <> 'issuer' AND a.balance < a.reserved) THEN
        RAISE EXCEPTION 'account funds below reservations or out of bounds' USING ERRCODE = '23514';
    END IF;
END;
$$;
CREATE OR REPLACE FUNCTION check_account_balance() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM assert_account_funds(NEW.account_id);
    RETURN NULL;
END;
$$;

-- Same lock order as the service: reservation (for terminal actions),
-- accounts in UUID order, then physical resource. Direct SQL obeys it too.
CREATE FUNCTION lock_reservation_dependencies() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r record; producer uuid;
BEGIN
    IF TG_TABLE_NAME = 'reservation_outcomes' THEN
        SELECT * INTO r FROM reservations WHERE id = NEW.reservation_id FOR UPDATE;
    ELSE
        r := NEW;
    END IF;
    SELECT producer_id INTO producer FROM resources WHERE id = r.resource_id;
    PERFORM 1 FROM accounts WHERE id IN (r.resident_id, producer) ORDER BY id FOR UPDATE;
    PERFORM 1 FROM resources WHERE id = r.resource_id FOR UPDATE;
    RETURN NEW;
END;
$$;
CREATE TRIGGER lock_reservation BEFORE INSERT ON reservations
    FOR EACH ROW EXECUTE FUNCTION lock_reservation_dependencies();
CREATE TRIGGER lock_outcome BEFORE INSERT ON reservation_outcomes
    FOR EACH ROW EXECUTE FUNCTION lock_reservation_dependencies();

CREATE FUNCTION check_reservation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r record; s record; result record;
BEGIN
    IF TG_TABLE_NAME = 'reservation_outcomes' THEN
        SELECT * INTO r FROM reservations WHERE id = NEW.reservation_id;
    ELSE
        r := NEW;
    END IF;
    SELECT * INTO s FROM resource_totals WHERE id = r.resource_id;
    IF (SELECT kind FROM accounts WHERE id = r.resident_id) <> 'resident'
       OR (SELECT kind FROM accounts WHERE id = s.producer_id) <> 'producer'
       OR r.amount::numeric <> r.quantity::numeric * s.unit_measure
       OR s.consumed + s.reserved > s.initial_quantity THEN
        RAISE EXCEPTION 'invalid reservation or insufficient stock' USING ERRCODE = '23514';
    END IF;
    PERFORM assert_account_funds(r.resident_id);
    SELECT * INTO result FROM reservation_outcomes WHERE reservation_id = r.id;
    IF result.state = 'settled' THEN
        IF (SELECT kind FROM journal_transactions WHERE id = result.transaction_id) <> 'settlement'
           OR (SELECT count(*) FROM postings WHERE transaction_id = result.transaction_id) <> 2
           OR NOT EXISTS (SELECT 1 FROM postings WHERE transaction_id = result.transaction_id AND account_id = r.resident_id AND delta = -r.amount)
           OR NOT EXISTS (SELECT 1 FROM postings WHERE transaction_id = result.transaction_id AND account_id = s.producer_id AND delta = r.amount) THEN
            RAISE EXCEPTION 'settlement must match reservation' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER valid_reservation AFTER INSERT ON reservations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_reservation();
CREATE CONSTRAINT TRIGGER valid_outcome AFTER INSERT ON reservation_outcomes
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_reservation();

CREATE FUNCTION check_allocation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (SELECT kind FROM accounts WHERE id = NEW.resident_id) <> 'resident'
       OR (SELECT kind FROM journal_transactions WHERE id = NEW.transaction_id) <> 'allocation'
       OR (SELECT count(*) FROM postings WHERE transaction_id = NEW.transaction_id) <> 2
       OR NOT EXISTS (SELECT 1 FROM postings WHERE transaction_id = NEW.transaction_id AND account_id = NEW.resident_id AND delta = NEW.amount)
       OR NOT EXISTS (SELECT 1 FROM postings p JOIN accounts a ON a.id = p.account_id WHERE p.transaction_id = NEW.transaction_id AND a.kind = 'treasury' AND p.delta = -NEW.amount) THEN
        RAISE EXCEPTION 'allocation must move treasury funds to resident' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER valid_allocation AFTER INSERT ON allocations
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_allocation();
