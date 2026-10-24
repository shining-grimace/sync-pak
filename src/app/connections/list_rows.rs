use std::{collections::HashMap, rc::Rc};

use slint::{Model, ModelRc, VecModel};

use crate::ConnectionRow;

// Activity refreshes own the summary IDs. Keep them while configuration and
// verification refreshes rebuild the other fields, including after filtering.
pub(super) fn update(
    existing: ModelRc<ConnectionRow>,
    mut rows: Vec<ConnectionRow>,
) -> ModelRc<ConnectionRow> {
    let summaries: HashMap<_, _> = existing.iter().map(|row| (row.id, row.result_id)).collect();
    for row in &mut rows {
        if let Some(result_id) = summaries.get(&row.id) {
            row.result_id = result_id.clone();
        }
    }
    if let Some(model) = existing.as_any().downcast_ref::<VecModel<ConnectionRow>>()
        && model.row_count() == rows.len()
        && model.iter().zip(&rows).all(|(old, new)| old.id == new.id)
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

#[cfg(test)]
mod tests;
