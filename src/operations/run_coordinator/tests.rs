use super::RunCoordinator;
use crate::{
    capabilities::CapabilityError,
    configuration::{ConfigStore, run_guard},
    operations::{
        execution::{ExecutionProgress, ExecutionResult, OperationExecutor},
        queue::{QueueEntry, QueueState, background::BackgroundQueue},
        transfer::progress::TransferProgressObserver,
    },
};
use std::{
    sync::{Arc, Condvar, Mutex, mpsc},
    time::Duration,
};

struct Executor {
    started: mpsc::SyncSender<()>,
    finish: (Mutex<bool>, Condvar),
}
impl OperationExecutor for Executor {
    fn execute(
        &self,
        entry: &QueueEntry,
        _: &dyn TransferProgressObserver,
    ) -> Result<ExecutionResult, CapabilityError> {
        assert!(entry.automatic_request.is_some());
        self.started.send(()).unwrap();
        let (lock, wake) = &self.finish;
        let done = lock.lock().unwrap();
        let (done, _) = wake
            .wait_timeout_while(done, Duration::from_secs(5), |done| !*done)
            .unwrap();
        assert!(*done, "test did not release worker");
        Ok(ExecutionProgress::new([]).finish())
    }
    fn cancel(&self, _: &str) -> Result<(), CapabilityError> {
        *self.finish.0.lock().unwrap() = true;
        self.finish.1.notify_one();
        Ok(())
    }
}
#[test]
fn automatic_work_is_independent_of_windows_and_protects_configuration_until_finished() {
    let directory =
        std::env::temp_dir().join(format!("syncpak-coordinator-{}", uuid::Uuid::new_v4()));
    let store = ConfigStore::at(directory.join("config.json"));
    let mut config = crate::configuration::lists::tests::config();
    config.connections[0].show_preview = false;
    store.save(&config).unwrap();
    let id = config.connections[0].id.as_str();
    let (started, receiver) = mpsc::sync_channel(1);
    let executor = Arc::new(Executor {
        started,
        finish: (Mutex::new(false), Condvar::new()),
    });
    let queue = Arc::new(
        BackgroundQueue::persistent(Arc::clone(&executor), None, directory.join("activity.json"))
            .unwrap(),
    );
    let coordinator = RunCoordinator::new(Arc::clone(&queue), store.path().into());
    let operation_id = coordinator.start_automatic(id).unwrap();
    receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(coordinator.start_automatic(id).is_err());
    assert_eq!(queue.activity()[0].state, QueueState::Running);
    assert!(run_guard::edit_provider(store.path(), config.providers[0].id.as_str()).is_err());
    let mut changed = config.clone();
    changed.connections[0].name = "Edited".into();
    assert!(store.save(&changed).is_err());
    changed = config.clone();
    changed.providers.clear();
    changed.connections.clear();
    assert!(store.save(&changed).is_err());
    changed = config.clone();
    changed.connections_newest_first = true;
    store.save(&changed).unwrap();
    executor.cancel(id).unwrap();
    drop(coordinator);
    let queue = Arc::try_unwrap(queue).ok().unwrap();
    drop(queue); // joins worker and records its result
    assert!(run_guard::active(store.path()).is_none());
    store.save(&config).unwrap();
    let restored =
        crate::operations::queue::OperationQueue::restore(directory.join("activity.json")).unwrap();
    let entry = restored.activity().next().unwrap();
    assert_eq!(entry.operation_id, operation_id);
    assert_eq!(entry.state, QueueState::Completed);
    assert!(entry.result.is_some());
    assert!(entry.automatic_request.is_none());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unattended_mirror_and_archive_require_one_direction_but_preview_can_allow_both() {
    use crate::configuration::{SyncMode, validate_run_policy};
    for mode in [SyncMode::Mirror, SyncMode::Archive] {
        assert!(validate_run_policy(mode, true, true, false).is_err());
        assert!(validate_run_policy(mode, true, true, true).is_ok());
        assert!(validate_run_policy(mode, true, false, false).is_ok());
        assert!(validate_run_policy(mode, false, true, false).is_ok());
    }
    assert!(validate_run_policy(SyncMode::AddOnly, true, true, false).is_ok());
    assert!(validate_run_policy(SyncMode::AddOnly, false, false, false).is_err());
}
