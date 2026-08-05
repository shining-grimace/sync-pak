use std::rc::Rc;

use slint::ComponentHandle;

use crate::{AppWindow, diagnostics_controller};

pub(crate) fn initialize(window: &AppWindow) {
    window.set_app_version(env!("CARGO_PKG_VERSION").into());
    let diagnostics = Rc::new(std::cell::RefCell::new(Default::default()));
    diagnostics_controller::configure(window, Rc::clone(&diagnostics));
    configure_navigation(window);
    crate::configuration_startup_controller::configure(window, diagnostics);
}

fn configure_navigation(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_show_welcome(move || set_page(&weak, 0));
    let weak = window.as_weak();
    window.on_show_privacy(move || show_privacy(&weak));
    let weak = window.as_weak();
    window.on_show_activity(move || set_page(&weak, 9));

    let weak = window.as_weak();
    window.on_request_navigation(move |page| request_navigation(&weak, page));

    let weak = window.as_weak();
    window.on_complete_pending_navigation(move || complete_pending_navigation(&weak));
}

fn show_privacy(weak: &slint::Weak<AppWindow>) {
    let Some(window) = weak.upgrade() else { return };
    let can_return_to_welcome = window.get_page() == 0
        || (window.get_page() == 3 && window.get_privacy_can_return_to_welcome());
    window.set_privacy_can_return_to_welcome(can_return_to_welcome);
    set_page(weak, 3);
}

fn set_page(weak: &slint::Weak<AppWindow>, page: i32) {
    if let Some(window) = weak.upgrade() {
        if window.get_configuration_unavailable() && !configuration_unavailable_allows(page) {
            return;
        }
        window.set_status_message(Default::default());
        window.set_notice_message(Default::default());
        window.set_page(page);
    }
}

fn request_navigation(weak: &slint::Weak<AppWindow>, page: i32) {
    let Some(window) = weak.upgrade() else { return };
    if navigation_is_blocked(window.get_page()) {
        return;
    }
    match window.get_page() {
        2 => {
            window.set_pending_navigation_page(page);
            window.invoke_request_discard_provider();
        }
        5 => {
            window.set_pending_navigation_page(page);
            window.invoke_request_discard_connection();
        }
        _ => navigate(&window, page),
    }
}

fn navigation_is_blocked(page: i32) -> bool {
    matches!(page, 6 | 7 | 12..=15)
}

fn configuration_unavailable_allows(page: i32) -> bool {
    matches!(page, 0 | 3)
}

fn complete_pending_navigation(weak: &slint::Weak<AppWindow>) {
    let Some(window) = weak.upgrade() else { return };
    let page = match window.get_pending_navigation_page() {
        page if page >= 0 => page,
        _ if window.get_page() == 2 || window.get_page() == 15 => 1,
        _ => 4,
    };
    window.set_pending_navigation_page(-1);
    navigate(&window, page);
}

fn navigate(window: &AppWindow, page: i32) {
    match page {
        1 => window.invoke_show_providers(),
        3 => window.invoke_show_privacy(),
        4 => window.invoke_show_connections(),
        9 => window.invoke_show_activity(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{configuration_unavailable_allows, navigation_is_blocked};

    #[test]
    fn navigation_cannot_bypass_confirmation_pages() {
        for page in [6, 7, 12, 13, 14, 15] {
            assert!(navigation_is_blocked(page));
        }
        for page in [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 16, 17] {
            assert!(!navigation_is_blocked(page));
        }
    }

    #[test]
    fn unavailable_configuration_only_allows_welcome_and_privacy() {
        assert!(configuration_unavailable_allows(0));
        assert!(configuration_unavailable_allows(3));
        assert!(!configuration_unavailable_allows(1));
        assert!(!configuration_unavailable_allows(9));
    }
}
