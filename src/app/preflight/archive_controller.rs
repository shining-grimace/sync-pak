use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{
    AppWindow, ArchiveReviewRow,
    inventory::{InventoryEntry, InventoryEntryKind, RelativePath},
    operations::request::RunRequest,
    preflight::{Preflight, reviewed_operation::ReviewedOperation},
};

use super::controller;

struct Selection {
    request: RunRequest,
    original: Preflight,
    excluded: BTreeSet<RelativePath>,
    rows: Rc<VecModel<ArchiveReviewRow>>,
}

thread_local! {
    static SELECTION: RefCell<Option<Selection>> = const { RefCell::new(None) };
}

pub(super) fn configure(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_set_archive_included(move |path, included| {
        let Some(window) = weak.upgrade() else { return };
        SELECTION.with(|state| {
            let mut state = state.borrow_mut();
            let Some(selection) = state.as_mut() else {
                return;
            };
            selection.set_included(path.as_str(), included);
            selection.publish(&window);
        });
    });
}

pub(super) fn begin(window: &AppWindow, request: RunRequest, preflight: &Preflight) {
    let rows = Rc::new(VecModel::from_iter(
        preflight
            .source()
            .entries()
            .filter(is_file)
            .map(|entry| ArchiveReviewRow {
                path: entry.path.as_str().into(),
                included: true,
            }),
    ));
    let selection = Selection {
        request,
        original: preflight.clone(),
        excluded: BTreeSet::new(),
        rows: rows.clone(),
    };
    window.set_preflight_archive_rows(ModelRc::new(rows));
    controller::show_review(window, preflight);
    selection.publish(window);
    SELECTION.with(|state| *state.borrow_mut() = Some(selection));
}

pub(super) fn clear() {
    SELECTION.with(|state| *state.borrow_mut() = None);
}

impl Selection {
    fn files(&self) -> impl Iterator<Item = &InventoryEntry> {
        self.original.source().entries().filter(is_file)
    }

    fn set_included(&mut self, path: &str, included: bool) {
        if path.is_empty() {
            self.excluded = if included {
                BTreeSet::new()
            } else {
                self.files().map(|entry| entry.path.clone()).collect()
            };
        } else if let Ok(path) = RelativePath::new(path)
            && self
                .original
                .source()
                .get(&path)
                .is_some_and(|entry| entry.kind != InventoryEntryKind::Directory)
        {
            if included {
                self.excluded.remove(&path);
            } else {
                self.excluded.insert(path);
            }
        }
    }

    fn publish(&self, window: &AppWindow) {
        for (index, entry) in self.files().enumerate() {
            let included = !self.excluded.contains(&entry.path);
            if let Some(mut row) = self.rows.row_data(index)
                && row.included != included
            {
                row.included = included;
                self.rows.set_row_data(index, row);
            }
        }
        let all_excluded =
            !self.excluded.is_empty() && self.excluded.len() == self.rows.row_count();
        window.set_preflight_archive_all_included(self.excluded.is_empty());
        window.set_preflight_has_planned_actions(!all_excluded);
        if all_excluded {
            window.set_preflight_selection_error(
                "Select at least one file to create an archive.".into(),
            );
            controller::set_reviewed(None);
        } else {
            window.set_preflight_selection_error("".into());
            controller::set_reviewed(Some(ReviewedOperation::new(
                self.request.clone(),
                self.original.with_archive_exclusions(self.excluded.clone()),
            )));
        }
    }
}

fn is_file(entry: &&InventoryEntry) -> bool {
    entry.kind != InventoryEntryKind::Directory
}
