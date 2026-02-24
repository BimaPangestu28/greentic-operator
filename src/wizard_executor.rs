use crate::wizard::{WizardExecutionReport, WizardMode, WizardPlan};

pub fn execute(
    mode: WizardMode,
    plan: &WizardPlan,
    offline: bool,
) -> anyhow::Result<WizardExecutionReport> {
    crate::wizard::execute_plan(mode, plan, offline)
}
