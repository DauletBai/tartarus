CREATE TABLE accounts (
    id uuid PRIMARY KEY,
    kind text NOT NULL CHECK (kind IN ('issuer', 'treasury', 'resident', 'producer')),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE UNIQUE INDEX single_issuer ON accounts (kind) WHERE kind = 'issuer';
CREATE UNIQUE INDEX single_treasury ON accounts (kind) WHERE kind = 'treasury';

CREATE TABLE journal_transactions (
    id uuid PRIMARY KEY,
    idempotency_key text NOT NULL UNIQUE CHECK (length(idempotency_key) BETWEEN 1 AND 128),
    kind text NOT NULL CHECK (kind IN ('transfer', 'genesis')),
    request jsonb NOT NULL,
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id() CHECK (created_xid = pg_current_xact_id()),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE postings (
    transaction_id uuid NOT NULL REFERENCES journal_transactions(id),
    account_id uuid NOT NULL REFERENCES accounts(id),
    delta bigint NOT NULL CHECK (delta <> 0),
    PRIMARY KEY (transaction_id, account_id)
);
CREATE INDEX postings_by_account ON postings(account_id);

-- Balances are derived from the journal; no separate mutable balance cache.
CREATE FUNCTION reject_history_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'ledger history is append only' USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER immutable_transactions BEFORE UPDATE OR DELETE OR TRUNCATE ON journal_transactions
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE TRIGGER immutable_postings BEFORE UPDATE OR DELETE OR TRUNCATE ON postings
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE TRIGGER immutable_accounts BEFORE UPDATE OR DELETE OR TRUNCATE ON accounts
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

-- The application locks all involved accounts in UUID order before inserting.
-- This lock also serializes callers that insert postings directly.
CREATE FUNCTION lock_posting_account() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM journal_transactions
        WHERE id = NEW.transaction_id AND created_xid = pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'postings must be inserted with their transaction' USING ERRCODE = '23514';
    END IF;
    PERFORM 1 FROM accounts WHERE id = NEW.account_id FOR UPDATE;
    RETURN NEW;
END;
$$;
CREATE TRIGGER lock_account BEFORE INSERT ON postings
    FOR EACH ROW EXECUTE FUNCTION lock_posting_account();

CREATE FUNCTION check_transaction_balanced() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    tx_id uuid;
    entry_count bigint;
    total numeric;
BEGIN
    IF TG_TABLE_NAME = 'journal_transactions' THEN
        tx_id := NEW.id;
    ELSE
        tx_id := NEW.transaction_id;
    END IF;
    SELECT count(*), COALESCE(sum(delta), 0) INTO entry_count, total
        FROM postings WHERE transaction_id = tx_id;
    IF entry_count < 2 OR total <> 0 THEN
        RAISE EXCEPTION 'unbalanced transaction' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER balanced_transaction AFTER INSERT ON journal_transactions
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_transaction_balanced();
CREATE CONSTRAINT TRIGGER balanced_postings AFTER INSERT ON postings
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_transaction_balanced();

CREATE FUNCTION check_account_balance() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    account_kind text;
    total numeric;
BEGIN
    SELECT kind INTO account_kind FROM accounts WHERE id = NEW.account_id;
    SELECT COALESCE(sum(delta), 0) INTO total FROM postings WHERE account_id = NEW.account_id;
    IF total > 9223372036854775807 OR total < -9223372036854775807
       OR (account_kind <> 'issuer' AND total < 0) THEN
        RAISE EXCEPTION 'account balance out of bounds' USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER bounded_balance AFTER INSERT ON postings
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_account_balance();

