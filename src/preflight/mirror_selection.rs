//! Interactive mirror choices, kept separate from unattended one-direction planning.
use std::collections::BTreeMap;

use crate::{
    configuration::SyncMode,
    inventory::{InventoryEntry, InventoryEntryKind, RelativePath},
    preflight::{
        Preflight,
        comparison::{ComparedEntry, EntryStatus},
        planning::{Direction, Endpoint, PlannedAction},
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MirrorChoice {
    Keep,
    Delete,
    UseLocal,
    UseRemote,
}

#[derive(Clone, Debug)]
pub struct MirrorSelection {
    original: Preflight,
    choices: BTreeMap<RelativePath, MirrorChoice>,
}

impl MirrorSelection {
    pub fn new(original: Preflight) -> Self {
        let mut selection = Self {
            original,
            choices: BTreeMap::new(),
        };
        if selection.original.plan().mode() == SyncMode::AddOnly {
            for entry in selection.original.comparison() {
                if selection.allowed(entry, MirrorChoice::Keep) {
                    selection
                        .choices
                        .insert(entry.path.clone(), MirrorChoice::Keep);
                }
            }
        }
        selection
    }

    pub fn original(&self) -> &Preflight {
        &self.original
    }

    pub fn choice(&self, path: &RelativePath) -> Option<MirrorChoice> {
        self.choices.get(path).copied()
    }

    pub fn choose(&mut self, path: &RelativePath, choice: Option<MirrorChoice>) -> bool {
        let Some(entry) = self
            .original
            .comparison()
            .iter()
            .find(|entry| entry.path == *path)
        else {
            return false;
        };
        if choice.is_some_and(|choice| !self.allowed(entry, choice)) {
            return false;
        }
        match choice {
            Some(choice) => {
                self.choices.insert(path.clone(), choice);
            }
            None => {
                self.choices.remove(path);
            }
        }
        true
    }

    pub fn allowed(&self, entry: &ComparedEntry, choice: MirrorChoice) -> bool {
        if self.original.plan().mode() == SyncMode::Archive
            || entry.status == EntryStatus::Unchanged
        {
            return false;
        }
        if self.original.plan().mode() == SyncMode::AddOnly && choice != MirrorChoice::Keep {
            return false;
        }
        let (local, remote) = self.sides(entry);
        let direction = self.original.plan().direction();
        let upload = direction != Direction::Download;
        let download = direction != Direction::Upload;
        match (local, remote, choice) {
            (Some(_), None, MirrorChoice::Keep) => upload,
            (None, Some(_), MirrorChoice::Keep) => download,
            (Some(_), None, MirrorChoice::Delete) => download,
            (None, Some(_), MirrorChoice::Delete) => upload,
            (Some(left), Some(right), MirrorChoice::UseLocal) => upload && compatible(left, right),
            (Some(left), Some(right), MirrorChoice::UseRemote) => {
                download && compatible(left, right)
            }
            _ => false,
        }
    }

    pub fn sides<'a>(
        &self,
        entry: &'a ComparedEntry,
    ) -> (Option<&'a InventoryEntry>, Option<&'a InventoryEntry>) {
        if self.original.plan().direction() == Direction::Download {
            (entry.destination.as_ref(), entry.source.as_ref())
        } else {
            (entry.source.as_ref(), entry.destination.as_ref())
        }
    }

    pub fn selected(&self) -> Result<Preflight, &'static str> {
        for entry in self.original.comparison() {
            if self.choice(&entry.path) == Some(MirrorChoice::Delete) {
                let prefix = format!("{}/", entry.path.as_str());
                if self.original.comparison().iter().any(|child| {
                    child.path.as_str().starts_with(&prefix)
                        && self.choice(&child.path) != Some(MirrorChoice::Delete)
                }) {
                    return Err("To delete a folder, select Delete for every entry inside it too.");
                }
            }
        }
        Ok(self.preview())
    }

    pub(crate) fn preview(&self) -> Preflight {
        let mut actions = Vec::new();
        for entry in self.original.comparison() {
            let Some(choice) = self.choice(&entry.path) else {
                continue;
            };
            let (local, _) = self.sides(entry);
            let local_endpoint = if self.original.plan().direction() == Direction::Download {
                Endpoint::Destination
            } else {
                Endpoint::Source
            };
            let remote_endpoint = opposite(local_endpoint);
            let from = match choice {
                MirrorChoice::Keep | MirrorChoice::Delete => {
                    if local.is_some() {
                        local_endpoint
                    } else {
                        remote_endpoint
                    }
                }
                MirrorChoice::UseLocal => local_endpoint,
                MirrorChoice::UseRemote => remote_endpoint,
            };
            let path = entry.path.clone();
            actions.push(match choice {
                MirrorChoice::Keep => PlannedAction::Copy {
                    path,
                    from,
                    to: opposite(from),
                },
                MirrorChoice::Delete => PlannedAction::Delete { path, from },
                _ => PlannedAction::Overwrite {
                    path,
                    from,
                    to: opposite(from),
                },
            });
        }
        if self.original.plan().mode() == SyncMode::AddOnly {
            actions.extend(
                self.original
                    .plan()
                    .actions()
                    .iter()
                    .filter(|action| matches!(action, PlannedAction::SkipChanged { .. }))
                    .cloned(),
            );
        }
        self.original.with_mirror_actions(actions)
    }
}

fn opposite(endpoint: Endpoint) -> Endpoint {
    match endpoint {
        Endpoint::Source => Endpoint::Destination,
        Endpoint::Destination => Endpoint::Source,
    }
}

fn compatible(left: &InventoryEntry, right: &InventoryEntry) -> bool {
    matches!(
        (&left.kind, &right.kind),
        (InventoryEntryKind::File, InventoryEntryKind::File)
    )
}

#[cfg(test)]
mod tests;
