use super::*;

impl OperationQueue {
    pub(crate) fn push_automatic(
        &mut self,
        request: crate::operations::request::RunRequest,
        snapshot: ActivitySnapshot,
        path: std::path::PathBuf,
    ) -> Uuid {
        let plan = OperationPlan::new(
            request.connection.id.as_str(),
            request.connection.mode,
            request.direction,
        );
        let id = self.push_entry(plan, snapshot, None);
        let entry = self.entries.back_mut().unwrap();
        entry.automatic_request = Some(request);
        entry.configuration_path = Some(path);
        id
    }

    pub(crate) fn set_configuration_path(&mut self, id: Uuid, path: std::path::PathBuf) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.operation_id == id) {
            entry.configuration_path = Some(path);
        }
    }
}
