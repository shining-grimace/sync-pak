use super::cancellation::release;
use super::*;

impl<E: OperationExecutor> Drop for BackgroundQueue<E> {
    fn drop(&mut self) {
        if let Some(connection_id) = self.active_connection() {
            let _ = self.executor.cancel(&connection_id);
        }
        self.stopping.store(true, Ordering::Release);
        self.queue.1.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        for entry in self
            .queue
            .0
            .lock()
            .expect("queue mutex poisoned")
            .activity()
        {
            if entry.state == crate::operations::queue::QueueState::Queued {
                release(entry);
            }
        }
        #[cfg(target_os = "android")]
        let _ = crate::platform::android::foreground_execution::clear_cancel_handler();
    }
}

impl<E: OperationExecutor> BackgroundQueue<E> {
    fn active_connection(&self) -> Option<String> {
        self.active_connection
            .lock()
            .ok()
            .and_then(|connection| connection.clone())
            .or_else(|| {
                self.queue.0.lock().ok().and_then(|queue| {
                    queue
                        .running()
                        .map(|entry| entry.plan.connection_id.clone())
                })
            })
    }
}

#[cfg(target_os = "android")]
pub(super) fn install_android_cancellation<E: OperationExecutor + Send + Sync + 'static>(
    executor: Arc<E>,
    active_connection: Arc<Mutex<Option<String>>>,
) {
    let handler = Arc::new(move || {
        let connection_id = active_connection
            .lock()
            .ok()
            .and_then(|connection| connection.clone());
        if let Some(connection_id) = connection_id {
            let _ = executor.cancel(&connection_id);
        }
    });
    let _ = crate::platform::android::foreground_execution::set_cancel_handler(handler);
}
