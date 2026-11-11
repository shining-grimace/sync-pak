use crate::{
    AppWindow, MirrorReviewRow,
    inventory::InventoryEntry,
    preflight::{
        comparison::{ComparedEntry, EntryStatus},
        mirror_selection::{MirrorChoice, MirrorSelection},
        planning::PlannedAction,
    },
};
use slint::{ModelRc, VecModel};
use std::rc::Rc;

pub(crate) fn section(entry: &ComparedEntry) -> i32 {
    if entry.source.is_none() || entry.destination.is_none() {
        0
    } else if entry.status == EntryStatus::Unchanged {
        2
    } else {
        1
    }
}

pub(crate) fn present(window: &AppWindow, selection: &MirrorSelection, interactive: bool) {
    let [isolated, conflicts, synced] = rows(selection, interactive);
    window.set_preflight_isolated(model(isolated));
    window.set_preflight_conflicts(model(conflicts));
    window.set_preflight_synced(model(synced));
}

fn rows(selection: &MirrorSelection, interactive: bool) -> [Vec<MirrorReviewRow>; 3] {
    let mut groups: [Vec<MirrorReviewRow>; 3] = Default::default();
    for entry in selection.original().comparison() {
        if section(entry) == 0
            && !selection.allowed(entry, MirrorChoice::Keep)
            && !selection.allowed(entry, MirrorChoice::Delete)
        {
            continue;
        }
        let (local, remote) = selection.sides(entry);
        let choice = if interactive {
            selection.choice(&entry.path)
        } else {
            automatic_choice(selection, entry)
        };
        let mut left = describe(local);
        let mut right = describe(remote);
        match choice {
            Some(MirrorChoice::Keep) => {
                if local.is_some() {
                    right = format!("{}\nWill copy from local", describe(local));
                } else {
                    left = format!("{}\nWill copy from remote", describe(remote));
                }
            }
            Some(MirrorChoice::Delete) => {
                if local.is_some() {
                    left = "Not present after deletion".into();
                } else {
                    right = "Not present after deletion".into();
                }
            }
            Some(MirrorChoice::UseLocal) => {
                right = format!("{}\nWill overwrite with local", describe(local));
            }
            Some(MirrorChoice::UseRemote) => {
                left = format!("{}\nWill overwrite with remote", describe(remote));
            }
            None => {}
        }
        if section(entry) == 1
            && !selection.allowed(entry, MirrorChoice::UseLocal)
            && !selection.allowed(entry, MirrorChoice::UseRemote)
            && interactive
        {
            left.push_str("\nEntry types cannot be reconciled automatically");
        }
        let isolated = section(entry) == 0;
        groups[section(entry) as usize].push(MirrorReviewRow {
            path: entry.path.as_str().into(),
            action_label: match choice {
                Some(MirrorChoice::Keep) => "Keep",
                Some(MirrorChoice::Delete) => "Delete",
                Some(MirrorChoice::UseLocal) => "Use Local",
                Some(MirrorChoice::UseRemote) => "Use Remote",
                None if section(entry) == 2 => "In Sync",
                None if selection.original().plan().mode()
                    == crate::configuration::SyncMode::AddOnly
                    && section(entry) == 1 =>
                {
                    "Skipped"
                }
                None => "No action",
            }
            .into(),
            deleting: choice == Some(MirrorChoice::Delete),
            local_detail: left.into(),
            remote_detail: right.into(),
            local_changed: choice.is_some(),
            remote_changed: choice.is_some(),
            selected: choice.is_some(),
            can_first: selection.allowed(
                entry,
                if isolated {
                    MirrorChoice::Keep
                } else {
                    MirrorChoice::UseLocal
                },
            ),
            can_second: selection.allowed(
                entry,
                if isolated {
                    MirrorChoice::Delete
                } else {
                    MirrorChoice::UseRemote
                },
            ),
        });
    }
    groups
}

fn model(rows: Vec<MirrorReviewRow>) -> ModelRc<MirrorReviewRow> {
    ModelRc::new(Rc::new(VecModel::from(rows)))
}

fn describe(entry: Option<&InventoryEntry>) -> String {
    let Some(entry) = entry else {
        return "Not present".into();
    };
    let time = entry
        .modified_unix_seconds
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        .map(|time| {
            time.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S %Z")
                .to_string()
        })
        .unwrap_or_else(|| "Modification time unavailable".into());
    match entry.kind {
        crate::inventory::InventoryEntryKind::Directory => format!("{time}\nFolder"),
        crate::inventory::InventoryEntryKind::Symlink { .. } => format!("{time}\nSymbolic link"),
        _ => format!("{time}\n{} bytes", entry.byte_size),
    }
}

fn automatic_choice(selection: &MirrorSelection, entry: &ComparedEntry) -> Option<MirrorChoice> {
    let action = selection
        .original()
        .plan()
        .actions()
        .iter()
        .find(|action| match action {
            PlannedAction::Copy { path, .. }
            | PlannedAction::Overwrite { path, .. }
            | PlannedAction::Delete { path, .. } => *path == entry.path,
            _ => false,
        })?;
    match action {
        PlannedAction::Copy { .. } => Some(MirrorChoice::Keep),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
