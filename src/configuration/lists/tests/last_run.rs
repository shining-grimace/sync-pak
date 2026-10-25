use super::config;
use crate::{
    configuration::{ConfigStore, ConnectionDraft, ConnectionRepository, run_guard},
    operations::{
        execution::{ExecutionProgress, ExecutionResult, OperationExecutor},
        queue::{QueueEntry, background::BackgroundQueue, snapshot::ActivitySnapshot},
        request::RunRequest,
        transfer::progress::TransferProgressObserver,
    },
};
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[test]
fn last_run_is_optional_and_persists_through_connection_edits() {
    let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let store = ConfigStore::at(directory.join("config.json"));
    let config = config();
    let c = &config.connections[0];
    store.save(&config).unwrap();
    assert_eq!(store.load().unwrap().connections[0].last_run, None);
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(store.path()).unwrap()).unwrap();
    assert!(json["connections"][0].get("last_run").is_none());

    store
        .record_connection_run(c.id.as_str(), 1_793_000_000)
        .unwrap();
    let updated = ConnectionRepository::new(&store)
        .update(
            &c.id,
            ConnectionDraft {
                name: "Renamed".into(),
                provider_id: c.provider_id.clone(),
                bucket: c.bucket.clone(),
                transport: c.transport.clone(),
                remote_path: c.remote_path.clone(),
                local_path: c.local_path.clone(),
                mode: c.mode,
                allow_upload: c.allow_upload,
                allow_download: c.allow_download,
                show_preview: c.show_preview,
                keep_last_archives: c.keep_last_archives,
                verified: c.verified,
            },
        )
        .unwrap();
    assert_eq!(updated.last_run, Some(1_793_000_000));
    let loaded = ConfigStore::at(store.path().into()).load().unwrap();
    assert_eq!(loaded.connections[0], updated);
    assert_eq!(loaded.schema_version, 1);
    std::fs::remove_dir_all(directory).unwrap();
}

struct Executor(bool);
impl OperationExecutor for Executor {
    fn execute(
        &self,
        _: &QueueEntry,
        _: &dyn TransferProgressObserver,
    ) -> Result<ExecutionResult, crate::CapabilityError> {
        Ok(if self.0 {
            ExecutionProgress::new([]).finish()
        } else {
            ExecutionResult::failed_before_start()
        })
    }
    fn cancel(&self, _: &str) -> Result<(), crate::CapabilityError> {
        Ok(())
    }
}

#[test]
fn worker_persists_last_run_for_success_and_failure_even_after_history_is_cleared() {
    for success in [true, false] {
        let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let store = ConfigStore::at(directory.join("config.json"));
        let mut config = config();
        config.connections[0].show_preview = false;
        store.save(&config).unwrap();
        let c = &config.connections[0];
        let request = RunRequest::automatic(&config, c.id.as_str()).unwrap();
        let snapshot = ActivitySnapshot::from_connection(c, "Cloud", request.direction);
        let queue = BackgroundQueue::new(Arc::new(Executor(success)));
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        run_guard::admit(store.path(), |state| {
            state.reserve(&request)?;
            queue.submit_automatic(request, snapshot, store.path().into());
            Ok(())
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while queue.activity().iter().all(|e| e.result.is_none()) {
            assert!(Instant::now() < deadline, "worker did not finish");
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(queue.take_history_error().is_none());
        queue.clear_completed();
        drop(queue);
        let timestamp = store.load().unwrap().connections[0].last_run.unwrap();
        assert!(timestamp >= before);
        assert!(
            timestamp
                <= SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
