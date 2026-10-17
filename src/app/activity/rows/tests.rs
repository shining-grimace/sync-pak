use super::*;
use slint::private_unstable_api::re_exports::{ModelChangeListener, ModelChangeListenerContainer};
use std::{cell::RefCell, pin::Pin};

#[derive(Debug, PartialEq)]
enum Change {
    Updated(usize),
    Added(usize, usize),
    Removed(usize, usize),
    Reset,
}

#[derive(Default)]
struct Observer(RefCell<Vec<Change>>);

impl ModelChangeListener for Observer {
    fn row_changed(self: Pin<&Self>, row: usize) {
        self.0.borrow_mut().push(Change::Updated(row));
    }
    fn row_added(self: Pin<&Self>, index: usize, count: usize) {
        self.0.borrow_mut().push(Change::Added(index, count));
    }
    fn row_removed(self: Pin<&Self>, index: usize, count: usize) {
        self.0.borrow_mut().push(Change::Removed(index, count));
    }
    fn reset(self: Pin<&Self>) {
        self.0.borrow_mut().push(Change::Reset);
    }
}

fn row(id: &str) -> ActivityRow {
    ActivityRow {
        id: id.into(),
        title: id.into(),
        ..Default::default()
    }
}

fn watch(model: &ModelRc<ActivityRow>) -> Pin<Box<ModelChangeListenerContainer<Observer>>> {
    let observer = Box::pin(ModelChangeListenerContainer::<Observer>::default());
    model
        .model_tracker()
        .attach_peer(observer.as_ref().model_peer());
    observer
}

#[test]
fn unchanged_refreshes_keep_the_model_and_emit_no_notifications() {
    let initial = vec![row("a"), row("b"), row("c")];
    let model = update(ModelRc::default(), initial.clone());
    let observer = watch(&model);
    for _ in 0..20 {
        assert_eq!(update(model.clone(), initial.clone()), model);
    }
    assert!(observer.0.borrow().is_empty());
}

#[test]
fn progress_updates_only_notify_the_changed_row() {
    let mut rows = vec![row("a"), row("b"), row("c")];
    let model = update(ModelRc::default(), rows.clone());
    let observer = watch(&model);
    rows[1].running = true;
    rows[1].progress = "Transferring file 2 of 10".into();
    rows[1].percent = 20;
    assert_eq!(update(model.clone(), rows.clone()), model);
    assert_eq!(model.iter().collect::<Vec<_>>(), rows);
    assert_eq!(*observer.0.borrow(), vec![Change::Updated(1)]);
}

#[test]
fn history_changes_preserve_retained_rows_without_resetting() {
    let model = update(ModelRc::default(), vec![row("a"), row("b"), row("c")]);
    let observer = watch(&model);
    let rows = vec![row("new"), row("a"), row("c")];
    assert_eq!(update(model.clone(), rows.clone()), model);
    assert_eq!(model.iter().collect::<Vec<_>>(), rows);
    assert_eq!(
        *observer.0.borrow(),
        vec![Change::Removed(1, 1), Change::Added(0, 1)]
    );
}

#[test]
fn clearing_then_repopulating_history_keeps_the_model() {
    let model = update(ModelRc::default(), vec![row("a"), row("b")]);
    let observer = watch(&model);
    assert_eq!(update(model.clone(), vec![]), model);
    assert_eq!(model.row_count(), 0);
    assert_eq!(update(model.clone(), vec![row("new")]), model);
    assert_eq!(model.row_data(0), Some(row("new")));
    assert_eq!(
        *observer.0.borrow(),
        vec![
            Change::Removed(1, 1),
            Change::Removed(0, 1),
            Change::Added(0, 1)
        ]
    );
}
