use super::*;
use slint::Model;

pub(super) fn filter_rows(rows: &ModelRc<PreflightRow>, filter: i32) -> ModelRc<PreflightRow> {
    ModelRc::new(Rc::new(VecModel::from_iter(
        rows.iter()
            .filter(|item| matches_filter(item.status.as_str(), filter)),
    )))
}

pub(super) fn matches_filter(status: &str, filter: i32) -> bool {
    match filter {
        0 => true,
        1 => status == "New",
        2 => status == "Will overwrite",
        3 => status == "Will delete",
        4 => status == "Warning",
        _ => false,
    }
}
