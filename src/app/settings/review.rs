use crate::configuration::{
    AppConfig, ConfigStore, ProviderId,
    lists::{ConflictChoice, ListDocument, import_list, local_path, portable, provider_candidates},
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

pub(super) type State = Rc<RefCell<Option<Review>>>;
pub(super) struct Review {
    pub original: AppConfig,
    pub document: ListDocument,
    pub local_root: String,
    pub mode: i32,
    pub selected: HashSet<String>,
    pub choices: BTreeMap<String, ConflictChoice>,
    pub mappings: BTreeMap<String, ProviderId>,
}
impl Review {
    pub fn new(original: AppConfig, document: Option<ListDocument>, mode: i32) -> Self {
        let document = document.unwrap_or_else(|| {
            ListDocument::new(
                original
                    .connections
                    .iter()
                    .filter_map(|c| portable(&original, c).ok())
                    .collect(),
            )
        });
        let selected = document
            .connections
            .iter()
            .map(|c| c.id.as_str().to_owned())
            .collect();
        let mut mappings = BTreeMap::new();
        for c in &document.connections {
            let candidates = provider_candidates(&original, &c.remote);
            if candidates.len() == 1 {
                mappings.insert(c.id.as_str().into(), candidates[0].id.clone());
            }
        }
        Self {
            local_root: original.local_root.clone(),
            original,
            document,
            mode,
            selected,
            choices: BTreeMap::new(),
            mappings,
        }
    }
    pub fn selected_document(&self) -> ListDocument {
        ListDocument::new(
            self.document
                .connections
                .iter()
                .filter(|c| self.selected.contains(c.id.as_str()))
                .cloned()
                .collect(),
        )
    }
    pub fn connection_id(&self, index: usize) -> Option<String> {
        if self.mode == 2 {
            self.original
                .connections
                .get(index)
                .map(|c| c.id.as_str().into())
        } else {
            self.document
                .connections
                .get(index)
                .map(|c| c.id.as_str().into())
        }
    }
    pub fn commit(&self, store: &ConfigStore) -> Result<usize, String> {
        let current = store.load().map_err(|e| e.to_string())?;
        if current != self.original {
            return Err("Settings changed during review. Cancel and reopen this review.".into());
        }
        let result = if self.mode == 3 {
            local_path(&self.local_root, "")?;
            let mut result = current;
            // Root is an export/import base, not a command to relocate configured folders.
            result.local_root = self.local_root.clone();
            result
        } else {
            if self.selected.is_empty() {
                return Err("Select at least one connection.".into());
            }
            import_list(
                &current,
                &self.selected_document(),
                &self.choices,
                &self.mappings,
            )?
        };
        let count = result
            .connections
            .iter()
            .filter(|c| !self.original.connections.contains(c))
            .count();
        store.save(&result).map_err(|e| e.to_string())?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests;
