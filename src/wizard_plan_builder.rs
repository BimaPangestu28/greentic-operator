use crate::wizard::{WizardCreateRequest, WizardMode, WizardPlan};

pub fn build_plan(
    mode: WizardMode,
    request: &WizardCreateRequest,
    dry_run: bool,
) -> anyhow::Result<WizardPlan> {
    let normalized = crate::wizard::normalize_request_for_plan(request)?;
    crate::wizard::apply(mode, &normalized, dry_run)
}
