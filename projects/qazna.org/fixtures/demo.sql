-- Synthetic initial state, NOT an issuance policy. Run only in an empty *_demo DB.
BEGIN;
DO $$
BEGIN
    IF right(current_database(), 5) <> '_demo' THEN
        RAISE EXCEPTION 'demo fixture requires a database ending in _demo';
    END IF;
    IF EXISTS (SELECT 1 FROM accounts) OR EXISTS (SELECT 1 FROM journal_transactions) THEN
        RAISE EXCEPTION 'demo fixture requires an empty ledger';
    END IF;
END;
$$;
INSERT INTO accounts(id, kind) VALUES
    ('00000000-0000-0000-0000-000000000001', 'issuer'),
    ('00000000-0000-0000-0000-000000000002', 'treasury'),
    ('00000000-0000-0000-0000-000000000003', 'resident');
INSERT INTO journal_transactions(id, idempotency_key, kind, request) VALUES
    ('00000000-0000-0000-0000-000000000010', 'demo-genesis-v1', 'genesis',
     '{"fixture":"demo-v1","units":"1000","synthetic":true}');
INSERT INTO postings(transaction_id, account_id, delta) VALUES
    ('00000000-0000-0000-0000-000000000010', '00000000-0000-0000-0000-000000000001', -1000),
    ('00000000-0000-0000-0000-000000000010', '00000000-0000-0000-0000-000000000002', 1000);
COMMIT;
