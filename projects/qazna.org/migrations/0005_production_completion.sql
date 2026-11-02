-- Existing lots stay valid but cannot be retrospectively labeled as new output.
ALTER TABLE resources ADD COLUMN created_xid xid8 NOT NULL DEFAULT pg_current_xact_id();
CREATE TABLE production_completions (
    plan_id uuid PRIMARY KEY REFERENCES production_plans(id),
    output_resource_id uuid UNIQUE REFERENCES resources(id) DEFERRABLE INITIALLY DEFERRED,
    actual_quantity bigint NOT NULL CHECK (actual_quantity >= 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    shortfall_basis text CHECK (length(shortfall_basis) BETWEEN 1 AND 1000),
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK ((actual_quantity=0) = (output_resource_id IS NULL))
);
CREATE TABLE production_input_losses (
    plan_id uuid NOT NULL REFERENCES production_completions(plan_id),
    slot text NOT NULL,
    quantity bigint NOT NULL CHECK (quantity > 0),
    PRIMARY KEY(plan_id,slot),
    FOREIGN KEY(plan_id,slot) REFERENCES production_plan_inputs(plan_id,slot)
);
CREATE TRIGGER immutable_completions BEFORE UPDATE OR DELETE OR TRUNCATE ON production_completions
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();
CREATE TRIGGER immutable_input_losses BEFORE UPDATE OR DELETE OR TRUNCATE ON production_input_losses
    FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change();

CREATE OR REPLACE VIEW production_capacity_totals AS
SELECT c.*, COALESCE((SELECT sum(p.batches::numeric*r.capacity_per_batch)
    FROM production_plans p JOIN production_recipes r ON r.id=p.recipe_id
    WHERE p.capacity_id=c.id AND NOT EXISTS(SELECT 1 FROM production_cancellations x WHERE x.plan_id=p.id)
    AND NOT EXISTS(SELECT 1 FROM production_completions x WHERE x.plan_id=p.id)),0) AS reserved,
    COALESCE((SELECT sum(p.batches::numeric*r.capacity_per_batch)
    FROM production_plans p JOIN production_recipes r ON r.id=p.recipe_id
    JOIN production_completions x ON x.plan_id=p.id WHERE p.capacity_id=c.id),0) AS consumed
FROM production_capacities c;
CREATE OR REPLACE VIEW resource_totals AS
SELECT s.id,s.producer_id,s.category,s.unit,s.initial_quantity,s.unit_measure,s.basis,
    COALESCE((SELECT sum(r.quantity) FROM reservations r
        JOIN reservation_outcomes o ON o.reservation_id=r.id AND o.state='settled'
        WHERE r.resource_id=s.id),0)
    + COALESCE((SELECT sum(i.quantity) FROM production_plan_inputs i
        JOIN production_completions x ON x.plan_id=i.plan_id WHERE i.resource_id=s.id),0) AS consumed,
    COALESCE((SELECT sum(r.quantity) FROM reservations r
        WHERE r.resource_id=s.id AND NOT EXISTS
        (SELECT 1 FROM reservation_outcomes o WHERE o.reservation_id=r.id)),0)
    + COALESCE((SELECT sum(i.quantity) FROM production_plan_inputs i
        WHERE i.resource_id=s.id AND NOT EXISTS
        (SELECT 1 FROM production_cancellations x WHERE x.plan_id=i.plan_id)
        AND NOT EXISTS(SELECT 1 FROM production_completions x WHERE x.plan_id=i.plan_id)),0) AS reserved
FROM resources s;

CREATE FUNCTION lock_production_terminal() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE target uuid;
BEGIN
    SELECT capacity_id INTO target FROM production_plans WHERE id=NEW.plan_id FOR UPDATE;
    IF EXISTS(SELECT 1 FROM production_completions WHERE plan_id=NEW.plan_id)
       OR EXISTS(SELECT 1 FROM production_cancellations WHERE plan_id=NEW.plan_id) THEN
        RAISE EXCEPTION 'production plan already closed' USING ERRCODE='23514';
    END IF;
    PERFORM 1 FROM production_capacities WHERE id=target FOR UPDATE;
    PERFORM 1 FROM resources WHERE id IN (SELECT resource_id FROM production_plan_inputs WHERE plan_id=NEW.plan_id) ORDER BY id FOR UPDATE;
    RETURN NEW;
END;
$$;
DROP TRIGGER lock_production_cancel ON production_cancellations;
CREATE TRIGGER lock_production_cancel BEFORE INSERT ON production_cancellations
    FOR EACH ROW EXECUTE FUNCTION lock_production_terminal();
CREATE TRIGGER lock_production_complete BEFORE INSERT ON production_completions
    FOR EACH ROW EXECUTE FUNCTION lock_production_terminal();

CREATE FUNCTION check_production_completion() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE result record; p record; r record; s record; target uuid; expected numeric;
BEGIN
    target:=NEW.plan_id;
    SELECT * INTO result FROM production_completions WHERE plan_id=target;
    SELECT * INTO p FROM production_plans WHERE id=target;
    SELECT * INTO r FROM production_recipes WHERE id=p.recipe_id;
    expected:=p.batches::numeric*r.output_per_batch;
    IF EXISTS(SELECT 1 FROM production_cancellations WHERE plan_id=target)
       OR p.created_xid=result.created_xid
       OR result.created_xid<>pg_current_xact_id()
       OR result.actual_quantity>expected
       OR (result.actual_quantity<expected AND result.shortfall_basis IS NULL)
       OR (TG_TABLE_NAME='production_input_losses' AND result.created_xid<>pg_current_xact_id())
       OR EXISTS(SELECT 1 FROM production_input_losses l JOIN production_plan_inputs i
                 ON i.plan_id=l.plan_id AND i.slot=l.slot WHERE l.plan_id=target AND l.quantity>i.quantity) THEN
        RAISE EXCEPTION 'invalid production completion or losses' USING ERRCODE='23514';
    END IF;
    IF result.output_resource_id IS NOT NULL THEN
        SELECT * INTO s FROM resources WHERE id=result.output_resource_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'completion output missing' USING ERRCODE='23514';
        END IF;
        IF s.created_xid<>result.created_xid OR s.category<>r.output_category OR s.unit<>r.output_unit
           OR s.initial_quantity<>result.actual_quantity
           OR NOT EXISTS(SELECT 1 FROM unit_providers WHERE producer_id=s.producer_id AND unit_id=r.unit_id)
           OR EXISTS(SELECT 1 FROM production_plan_inputs WHERE plan_id=target AND resource_id=s.id) THEN
            RAISE EXCEPTION 'output must be new, match recipe and belong to executing unit' USING ERRCODE='23514';
        END IF;
    END IF;
    IF EXISTS(SELECT 1 FROM production_capacity_totals WHERE id=p.capacity_id AND reserved+consumed>quantity)
       OR EXISTS(SELECT 1 FROM resource_totals WHERE id IN (SELECT resource_id FROM production_plan_inputs WHERE plan_id=target)
                 AND consumed+reserved>initial_quantity) THEN
        RAISE EXCEPTION 'completion exceeds physical limits' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER valid_completion AFTER INSERT ON production_completions
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_production_completion();
CREATE CONSTRAINT TRIGGER valid_input_loss AFTER INSERT ON production_input_losses
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_production_completion();

CREATE OR REPLACE FUNCTION check_production_plan() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p record; r record; c record; target uuid;
BEGIN
    IF TG_TABLE_NAME='production_plans' THEN target:=NEW.id; ELSE target:=NEW.plan_id; END IF;
    SELECT * INTO p FROM production_plans WHERE id=target;
    SELECT * INTO r FROM production_recipes WHERE id=p.recipe_id;
    SELECT * INTO c FROM production_capacity_totals WHERE id=p.capacity_id;
    IF (TG_TABLE_NAME='production_plan_inputs' AND p.created_xid<>pg_current_xact_id())
       OR r.unit_id<>c.unit_id OR r.capacity_unit<>c.capacity_unit OR c.reserved+c.consumed>c.quantity
       OR p.batches::numeric*r.output_per_batch>9223372036854775807
       OR p.batches::numeric*r.capacity_per_batch>9223372036854775807
       OR (SELECT count(*) FROM production_plan_inputs WHERE plan_id=p.id)
          <> (SELECT count(*) FROM production_recipe_inputs WHERE recipe_id=r.id)
       OR EXISTS (
           SELECT 1 FROM production_plan_inputs i
           LEFT JOIN production_recipe_inputs q ON q.recipe_id=r.id AND q.slot=i.slot
           JOIN resource_totals s ON s.id=i.resource_id
           WHERE i.plan_id=p.id AND (q.slot IS NULL OR s.category<>q.category OR s.unit<>q.unit
             OR i.quantity::numeric<>p.batches::numeric*q.quantity_per_batch
             OR s.consumed+s.reserved>s.initial_quantity
             OR NOT EXISTS(SELECT 1 FROM unit_providers u WHERE u.producer_id=s.producer_id))) THEN
        RAISE EXCEPTION 'invalid production plan or insufficient capacity/inputs' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;
