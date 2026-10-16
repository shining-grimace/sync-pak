use super::{CapabilityError, OperationProgress};

/// Keeps the host informed while an operation continues outside the foreground UI.
pub trait BackgroundExecution {
    fn start(&self, connection_name: &str) -> Result<(), CapabilityError>;
    fn update(
        &self,
        connection_name: &str,
        progress: &OperationProgress,
    ) -> Result<(), CapabilityError>;
    fn stop(&self) -> Result<(), CapabilityError>;
    /// Ends background execution and reports an outcome without requiring an open window.
    fn finish(&self, _connection_name: &str, _summary: &str) -> Result<(), CapabilityError> {
        self.stop()
    }
}
