-- Planning holds real inputs and declared capacity; it does not create output or money.
CREATE TABLE production_capacities (
    id uuid PRIMARY KEY,
    unit_id uuid NOT NULL REFERENCES provisioning_units(id),
    period date NOT NULL,
    capacity_unit text NOT NULL CHECK (length(capacity_unit) BETWEEN 1 AND 40),
    quantity bigint NOT NULL CHECK (quantity > 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    UNIQUE(unit_id,period,capacity_unit)
);
CREATE TABLE production_recipes (
    id uuid PRIMARY KEY,
    unit_id uuid NOT NULL REFERENCES provisioning_units(id),
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    capacity_unit text NOT NULL CHECK (length(capacity_unit) BETWEEN 1 AND 40),
    capacity_per_batch bigint NOT NULL CHECK (capacity_per_batch > 0),
    output_category text NOT NULL CHECK (output_category IN ('food','water','energy')),
    output_unit text NOT NULL CHECK (length(output_unit) BETWEEN 1 AND 40),
    output_per_batch bigint NOT NULL CHECK (output_per_batch > 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE production_recipe_inputs (
    recipe_id uuid NOT NULL REFERENCES production_recipes(id),
    slot text NOT NULL CHECK (length(slot) BETWEEN 1 AND 40),
    category text NOT NULL CHECK (category IN ('food','water','energy')),
    unit text NOT NULL CHECK (length(unit) BETWEEN 1 AND 40),
    quantity_per_batch bigint NOT NULL CHECK (quantity_per_batch > 0),
    PRIMARY KEY(recipe_id,slot)
);
CREATE FUNCTION check_recipe_input() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (SELECT created_xid FROM production_recipes WHERE id=NEW.recipe_id) <> pg_current_xact_id()
       OR (SELECT count(*) FROM production_recipe_inputs WHERE recipe_id=NEW.recipe_id) > 32 THEN
        RAISE EXCEPTION 'recipe inputs must be created with recipe, maximum 32' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END;
$$;
CREATE CONSTRAINT TRIGGER valid_recipe_input AFTER INSERT ON production_recipe_inputs
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_recipe_input();
CREATE TABLE production_plans (
    id uuid PRIMARY KEY,
    recipe_id uuid NOT NULL REFERENCES production_recipes(id),
    capacity_id uuid NOT NULL REFERENCES production_capacities(id),
    batches bigint NOT NULL CHECK (batches > 0),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    created_xid xid8 NOT NULL DEFAULT pg_current_xact_id(),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX production_plans_capacity ON production_plans(capacity_id);
CREATE TABLE production_plan_inputs (
    plan_id uuid NOT NULL REFERENCES production_plans(id),
    slot text NOT NULL,
    resource_id uuid NOT NULL REFERENCES resources(id),
    quantity bigint NOT NULL CHECK (quantity > 0),
    PRIMARY KEY(plan_id,slot),
    UNIQUE(plan_id,resource_id)
);
CREATE INDEX production_inputs_resource ON production_plan_inputs(resource_id);
CREATE TABLE production_cancellations (
    plan_id uuid PRIMARY KEY REFERENCES production_plans(id),
    basis text NOT NULL CHECK (length(basis) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
DO $$ DECLARE t text; BEGIN
    FOREACH t IN ARRAY ARRAY['production_capacities','production_recipes','production_recipe_inputs','production_plans','production_plan_inputs','production_cancellations'] LOOP
        EXECUTE format('CREATE TRIGGER immutable_history BEFORE UPDATE OR DELETE OR TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION reject_history_change()', t);
    END LOOP;
END $$;
CREATE VIEW production_capacity_totals AS
SELECT c.*, COALESCE((SELECT sum(p.batches::numeric*r.capacity_per_batch)
    FROM production_plans p JOIN production_recipes r ON r.id=p.recipe_id
    WHERE p.capacity_id=c.id AND NOT EXISTS(SELECT 1 FROM production_cancellations x WHERE x.plan_id=p.id)),0) AS reserved
FROM production_capacities c;
CREATE OR REPLACE VIEW resource_totals AS
SELECT s.*,
    COALESCE((SELECT sum(r.quantity) FROM reservations r
        JOIN reservation_outcomes o ON o.reservation_id=r.id AND o.state='settled'
        WHERE r.resource_id=s.id),0) AS consumed,
    COALESCE((SELECT sum(r.quantity) FROM reservations r
        WHERE r.resource_id=s.id AND NOT EXISTS
        (SELECT 1 FROM reservation_outcomes o WHERE o.reservation_id=r.id)),0)
    + COALESCE((SELECT sum(i.quantity) FROM production_plan_inputs i
        WHERE i.resource_id=s.id AND NOT EXISTS
        (SELECT 1 FROM production_cancellations x WHERE x.plan_id=i.plan_id)),0) AS reserved
FROM resources s;
CREATE FUNCTION lock_production_dependencies() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE target uuid;
BEGIN
    IF TG_TABLE_NAME='production_plans' THEN
        PERFORM 1 FROM production_capacities WHERE id=NEW.capacity_id FOR UPDATE;
    ELSIF TG_TABLE_NAME='production_plan_inputs' THEN
        PERFORM 1 FROM resources WHERE id=NEW.resource_id FOR UPDATE;
    ELSE
        SELECT capacity_id INTO target FROM production_plans WHERE id=NEW.plan_id FOR UPDATE;
        PERFORM 1 FROM production_capacities WHERE id=target FOR UPDATE;
        PERFORM 1 FROM resources WHERE id IN (SELECT resource_id FROM production_plan_inputs WHERE plan_id=NEW.plan_id) ORDER BY id FOR UPDATE;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER lock_production_plan BEFORE INSERT ON production_plans FOR EACH ROW EXECUTE FUNCTION lock_production_dependencies();
CREATE TRIGGER lock_production_input BEFORE INSERT ON production_plan_inputs FOR EACH ROW EXECUTE FUNCTION lock_production_dependencies();
CREATE TRIGGER lock_production_cancel BEFORE INSERT ON production_cancellations FOR EACH ROW EXECUTE FUNCTION lock_production_dependencies();
CREATE FUNCTION check_production_plan() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p record; r record; c record; target uuid;
BEGIN
    IF TG_TABLE_NAME='production_plans' THEN target:=NEW.id; ELSE target:=NEW.plan_id; END IF;
    SELECT * INTO p FROM production_plans WHERE id=target;
    SELECT * INTO r FROM production_recipes WHERE id=p.recipe_id;
    SELECT * INTO c FROM production_capacity_totals WHERE id=p.capacity_id;
    IF (TG_TABLE_NAME='production_plan_inputs' AND p.created_xid<>pg_current_xact_id())
       OR r.unit_id<>c.unit_id OR r.capacity_unit<>c.capacity_unit OR c.reserved>c.quantity
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
CREATE CONSTRAINT TRIGGER valid_production_plan AFTER INSERT ON production_plans
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_production_plan();
CREATE CONSTRAINT TRIGGER valid_production_input AFTER INSERT ON production_plan_inputs
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_production_plan();
