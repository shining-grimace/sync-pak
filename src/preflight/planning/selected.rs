use super::*;

impl TransferPlan {
    pub(crate) fn selected_mirror(
        mode: SyncMode,
        direction: Direction,
        actions: Vec<PlannedAction>,
        comparison: &[ComparedEntry],
    ) -> Self {
        Self {
            mode,
            direction,
            summary: summarize(&actions, comparison),
            actions,
        }
    }
}
