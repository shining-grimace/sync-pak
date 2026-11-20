use crate::{
    configuration::SyncMode,
    preflight::planning::{Direction, TransferPlan},
};

pub(super) fn start_action(plan: &TransferPlan) -> &'static str {
    if plan.actions().is_empty() {
        return "Accept Review";
    }
    match plan.mode() {
        SyncMode::Archive => "Create Archive",
        SyncMode::Mirror => "Start Mirror",
        SyncMode::AddOnly => match plan.direction() {
            Direction::Upload => "Start Upload",
            Direction::Download => "Start Download",
            Direction::BothWays => "Start Both Ways",
        },
    }
}
