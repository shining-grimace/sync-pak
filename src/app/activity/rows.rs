use crate::ActivityRow;
use slint::{Model, ModelRc, VecModel};
use std::{collections::HashSet, rc::Rc};

/// Preserve the list's row instances and scroll state across background refreshes.
pub(super) fn update(
    existing: ModelRc<ActivityRow>,
    rows: Vec<ActivityRow>,
) -> ModelRc<ActivityRow> {
    let Some(model) = existing.as_any().downcast_ref::<VecModel<ActivityRow>>() else {
        return ModelRc::new(Rc::new(VecModel::from(rows)));
    };

    let wanted: HashSet<_> = rows.iter().map(|row| row.id.as_str()).collect();
    for index in (0..model.row_count()).rev() {
        if model
            .row_data(index)
            .is_some_and(|row| !wanted.contains(row.id.as_str()))
        {
            model.remove(index);
        }
    }

    for (index, row) in rows.iter().enumerate() {
        let current = model.row_data(index);
        if current.as_ref().is_some_and(|current| current.id == row.id) {
            if current.as_ref() != Some(row) {
                model.set_row_data(index, row.clone());
            }
        } else {
            if let Some(previous) = (index + 1..model.row_count()).find(|&position| {
                model
                    .row_data(position)
                    .is_some_and(|item| item.id == row.id)
            }) {
                model.remove(previous);
            }
            model.insert(index, row.clone());
        }
    }
    existing
}

#[cfg(test)]
mod tests;
