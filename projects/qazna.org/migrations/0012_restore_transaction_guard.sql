-- Transaction-dependent checks must not run while loading historical dump rows.
-- pg_dump installs triggers after data, then normal writes use this same guard.
ALTER TABLE journal_transactions DROP CONSTRAINT journal_transactions_created_xid_check;
CREATE FUNCTION check_new_transaction_xid() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xid<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'transaction must use its current transaction ID' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER current_transaction_xid BEFORE INSERT ON journal_transactions
    FOR EACH ROW EXECUTE FUNCTION check_new_transaction_xid();
