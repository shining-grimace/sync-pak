//! Session cache for provider-supplied display labels; all URI access stays off the UI thread.
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    // None means a query is already in flight.
    static LABELS: RefCell<HashMap<String, Option<String>>> = RefCell::new(HashMap::new());
}

pub(super) fn label(path: &str) -> String {
    LABELS
        .with(|labels| labels.borrow().get(path).cloned().flatten())
        .unwrap_or_else(|| super::local_path_label::label(path))
}

#[cfg(target_os = "android")]
pub(super) fn resolve(weak: &slint::Weak<crate::AppWindow>) {
    use slint::Model;
    let Some(window) = weak.upgrade() else { return };
    let pending = LABELS.with(|labels| {
        let mut labels = labels.borrow_mut();
        window
            .get_connections()
            .iter()
            .filter_map(|row| {
                let uri = row.local_path_source.to_string();
                if !uri.starts_with("content://")
                    || uri.starts_with("content://com.android.externalstorage.documents/tree/")
                    || labels.contains_key(&uri)
                {
                    return None;
                }
                labels.insert(uri.clone(), None);
                Some(uri)
            })
            .collect::<Vec<_>>()
    });
    if pending.is_empty() {
        return;
    }
    let weak = weak.clone();
    std::thread::spawn(move || {
        for uri in pending {
            let label = crate::platform::android::document_tree::access::display_label(&uri)
                .ok()
                .filter(|label| !label.trim().is_empty())
                .unwrap_or_else(|| super::local_path_label::label(&uri));
            let weak = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                LABELS.with(|labels| labels.borrow_mut().insert(uri.clone(), Some(label.clone())));
                let Some(window) = weak.upgrade() else { return };
                let rows = window.get_connections();
                for index in 0..rows.row_count() {
                    let Some(mut row) = rows.row_data(index) else {
                        continue;
                    };
                    // An edit or filter change may have replaced the row during the query.
                    if row.local_path_source == uri {
                        row.local_path = label.clone().into();
                        rows.set_row_data(index, row);
                    }
                }
            });
        }
    });
}
