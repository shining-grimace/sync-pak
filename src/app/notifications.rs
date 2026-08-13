use std::{cell::Cell, rc::Rc, time::Duration};

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{AppNotification, AppWindow};

const TRANSIENT_DELAY: Duration = Duration::from_secs(2);
const FADE_DURATION: Duration = Duration::from_millis(500);
const DIAGNOSTICS_ACTION: i32 = 1;

pub(crate) fn configure(window: &AppWindow) {
    let notifications = Rc::new(VecModel::default());
    window.set_notifications(ModelRc::from(notifications.clone()));
    let next_id = Rc::new(Cell::new(1));

    let weak = window.as_weak();
    let queue = notifications.clone();
    let ids = next_id.clone();
    window.on_queue_notification(move |message, kind, action| {
        if message.is_empty() {
            return;
        }
        let id = ids.get();
        ids.set(id + 1);
        let height = height_for(&message, action);
        queue.push(AppNotification {
            id,
            message,
            kind,
            action,
            fading: false,
            height,
            offset: Default::default(),
        });
        relayout(&queue);
        if action == 0 {
            fade_after(weak.clone(), queue.clone(), id);
        }
    });

    let queue = notifications.clone();
    window.on_dismiss_notification(move |id| fade(&queue, id));

    let queue = notifications;
    let weak = window.as_weak();
    window.on_activate_notification(move |id| {
        let action = find(&queue, id)
            .and_then(|row| queue.row_data(row))
            .map(|notice| notice.action);
        if action == Some(DIAGNOSTICS_ACTION) {
            if let Some(window) = weak.upgrade() {
                window.invoke_show_diagnostics();
            }
        }
        fade(&queue, id);
    });
}

pub(crate) fn present_diagnostic(window: &AppWindow, message: &'static str) {
    window.invoke_queue_notification(message.into(), 0, DIAGNOSTICS_ACTION);
}

fn fade_after(weak: slint::Weak<AppWindow>, queue: Rc<VecModel<AppNotification>>, id: i32) {
    slint::Timer::single_shot(TRANSIENT_DELAY, move || {
        fade(&queue, id);
        let _ = weak;
    });
}

fn fade(queue: &Rc<VecModel<AppNotification>>, id: i32) {
    let Some(row) = find(queue, id) else { return };
    let mut notice = queue.row_data(row).expect("notification row exists");
    if notice.fading {
        return;
    }
    notice.fading = true;
    queue.set_row_data(row, notice);
    relayout(queue);
    let queue = queue.clone();
    slint::Timer::single_shot(FADE_DURATION, move || remove(&queue, id));
}

fn find(queue: &VecModel<AppNotification>, id: i32) -> Option<usize> {
    (0..queue.row_count()).find(|&row| queue.row_data(row).is_some_and(|item| item.id == id))
}

fn remove(queue: &VecModel<AppNotification>, id: i32) {
    if let Some(row) = find(queue, id) {
        queue.remove(row);
        relayout(queue);
    }
}

fn height_for(message: &str, action: i32) -> f32 {
    let lines = (message.chars().count().div_ceil(48)).max(1) as f32;
    let action_height = if action == 0 { 0.0 } else { 38.0 };
    56.0 + lines * 22.0 + action_height
}

fn relayout(queue: &VecModel<AppNotification>) {
    let mut offset = 0.0;
    for row in 0..queue.row_count() {
        let mut notice = queue.row_data(row).expect("notification row exists");
        notice.offset = offset;
        offset += notice.height + 8.0;
        queue.set_row_data(row, notice);
    }
}
