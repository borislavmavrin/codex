use codex_protocol::plan_tool::PlanItemArg;
use codex_protocol::plan_tool::StepStatus;
use codex_protocol::plan_tool::UpdatePlanArgs;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct PlanOutput {
    pub explanation: Option<String>,
    pub plan: Vec<PlanItemArg>,
    pub summary: String,
}

pub fn update_plan(args: UpdatePlanArgs) -> anyhow::Result<PlanOutput> {
    let UpdatePlanArgs { explanation, plan } = args;

    // Count statuses
    let mut pending = 0;
    let mut in_progress = 0;
    let mut completed = 0;

    for item in &plan {
        match item.status {
            StepStatus::Pending => pending += 1,
            StepStatus::InProgress => in_progress += 1,
            StepStatus::Completed => completed += 1,
        }
    }

    let total = plan.len();
    let summary = format!(
        "Plan updated: {total} total steps ({completed} completed, {in_progress} in progress, {pending} pending)"
    );

    Ok(PlanOutput {
        explanation,
        plan,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_plan_output() {
        let args = UpdatePlanArgs {
            explanation: Some("Testing plan".to_string()),
            plan: vec![
                PlanItemArg {
                    step: "Step 1".to_string(),
                    status: StepStatus::Completed,
                },
                PlanItemArg {
                    step: "Step 2".to_string(),
                    status: StepStatus::InProgress,
                },
                PlanItemArg {
                    step: "Step 3".to_string(),
                    status: StepStatus::Pending,
                },
            ],
        };

        let output = update_plan(args).unwrap();

        assert_eq!(output.explanation, Some("Testing plan".to_string()));
        assert_eq!(output.plan.len(), 3);
        assert!(output.summary.contains("3 total steps"));
        assert!(output.summary.contains("1 completed"));
        assert!(output.summary.contains("1 in progress"));
        assert!(output.summary.contains("1 pending"));
    }
}
