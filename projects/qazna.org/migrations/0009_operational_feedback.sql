-- Active commitments are forecasts, never receipt or available stock.
CREATE VIEW need_production_forecasts AS
SELECT a.need_id,p.id AS plan_id,p.capacity_id,
    (d.proposal->'needs'->a.need_position->'production'->a.action_position->>'expected_for_need')::numeric AS quantity
FROM planning_actions a JOIN planning_drafts d ON d.id=a.draft_id
JOIN production_plans p ON p.id=a.production_plan_id
WHERE a.kind='production'
  AND NOT EXISTS(SELECT 1 FROM production_cancellations x WHERE x.plan_id=p.id)
  AND NOT EXISTS(SELECT 1 FROM production_completions x WHERE x.plan_id=p.id);
CREATE INDEX planning_actions_need_production ON planning_actions(need_id) WHERE kind='production';
ALTER TABLE coordination_results DROP CONSTRAINT coordination_results_engine_check;
ALTER TABLE coordination_results ADD CONSTRAINT coordination_results_engine_check
    CHECK (engine IN ('ordered-planner-v1','ordered-planner-v2'));
