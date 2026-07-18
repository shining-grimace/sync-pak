use crate::configuration::SyncMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Upload,
    Download,
    BothWays,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationPlan {
    pub connection_id: String,
    pub mode: SyncMode,
    pub direction: Direction,
}

impl OperationPlan {
    pub fn new(connection_id: impl Into<String>, mode: SyncMode, direction: Direction) -> Self {
        Self {
            connection_id: connection_id.into(),
            mode,
            direction,
        }
    }
}
