use super::review::{Review, State};
use crate::{
    AppWindow, SettingsConnectionRow, SettingsState,
    configuration::{
        ConfigStore,
        lists::{ConflictChoice, local_path, portable, provider_candidates},
    },
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::rc::Rc;

fn update<T: Clone + PartialEq + 'static>(
    existing: ModelRc<T>,
    rows: impl IntoIterator<Item = T>,
) -> ModelRc<T> {
    let rows: Vec<_> = rows.into_iter().collect();
    if let Some(model) = existing.as_any().downcast_ref::<VecModel<T>>()
        && model.row_count() == rows.len()
    {
        for (index, row) in rows.into_iter().enumerate() {
            if model.row_data(index).as_ref() != Some(&row) {
                model.set_row_data(index, row);
            }
        }
        return existing;
    }
    ModelRc::new(Rc::new(VecModel::from(rows)))
}

pub(crate) fn refresh(window: &AppWindow) {
    crate::app::privacy::refresh(window);
    let state = window.global::<SettingsState>();
    match ConfigStore::for_current_platform().and_then(|store| store.load()) {
        Ok(config) => {
            state.set_local_root(config.local_root.into());
            state.set_has_connections(!config.connections.is_empty());
        }
        Err(error) => state.set_error(error.to_string().into()),
    }
}

pub(super) fn review(window: &AppWindow, review: &State) {
    let borrowed = review.borrow();
    let Some(review) = borrowed.as_ref() else {
        return;
    };
    let state = window.global::<SettingsState>();
    state.set_mode(review.mode);
    state.set_local_root(review.local_root.clone().into());
    state.set_root_error(
        local_path(&review.local_root, "")
            .err()
            .unwrap_or_default()
            .into(),
    );
    state.set_provider_names(update(
        state.get_provider_names(),
        std::iter::once(SharedString::from("Choose a provider")).chain(
            review
                .original
                .providers
                .iter()
                .map(|p| SharedString::from(p.name.as_str())),
        ),
    ));
    let rows = if review.mode == 2 {
        review
            .original
            .connections
            .iter()
            .map(|c| match portable(&review.original, c) {
                Ok(portable) => row(review, &portable),
                Err(error) => SettingsConnectionRow {
                    id: c.id.as_str().into(),
                    name: c.name.clone().into(),
                    local: error.clone().into(),
                    error: error.into(),
                    compatible: false,
                    ..Default::default()
                },
            })
            .collect::<Vec<_>>()
    } else if review.mode == 1 {
        review
            .document
            .connections
            .iter()
            .map(|c| row(review, c))
            .collect()
    } else {
        vec![]
    };
    state.set_connections(update(state.get_connections(), rows));
}

fn row(
    review: &Review,
    c: &crate::configuration::lists::PortableConnection,
) -> SettingsConnectionRow {
    let provider = review
        .mappings
        .get(c.id.as_str())
        .and_then(|id| review.original.providers.iter().position(|p| &p.id == id))
        .map(|i| i as i32 + 1)
        .unwrap_or(0);
    let mut error = String::new();
    if review.mode == 1 && review.selected.contains(c.id.as_str()) {
        let candidates = provider_candidates(&review.original, &c.remote);
        if provider == 0
            || !candidates
                .iter()
                .any(|p| Some(&p.id) == review.mappings.get(c.id.as_str()))
        {
            error = "Choose a matching provider for this bucket.".into();
        }
    }
    SettingsConnectionRow {
        id: c.id.as_str().into(),
        name: c.name.clone().into(),
        detail: format!(
            "{:?} • {}",
            c.mode,
            match (c.allow_upload, c.allow_download) {
                (true, true) => "Upload and download",
                (true, false) => "Upload only",
                _ => "Download only",
            }
        )
        .into(),
        local: if c.local_path.is_empty() {
            "(root)".into()
        } else {
            format!("(root)/{}", c.local_path).into()
        },
        remote: format!(
            "{:?} / {} / {}",
            c.remote.provider_kind, c.remote.bucket, c.remote.path
        )
        .into(),
        error: error.into(),
        compatible: true,
        provider,
        selected: review.selected.contains(c.id.as_str()),
        conflict: review
            .original
            .connections
            .iter()
            .any(|saved| saved.id == c.id),
        choice: match review
            .choices
            .get(c.id.as_str())
            .copied()
            .unwrap_or_default()
        {
            ConflictChoice::Skip => 0,
            ConflictChoice::Replace => 1,
            ConflictChoice::Copy => 2,
        },
    }
}
