use std::time::Duration;

use slint::ComponentHandle;

use crate::AppWindow;

const REVEAL_DURATION: Duration = Duration::from_secs(15);

/// Makes credential reveal brief and invalidates older timers on every toggle.
pub(crate) fn configure(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_toggle_provider_secret_visibility(move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_provider_secret_visible() {
            hide(&window);
        } else {
            let generation = next_generation(window.get_provider_secret_reveal_generation());
            window.set_provider_secret_reveal_generation(generation);
            window.set_provider_secret_visible(true);
            hide_after_timeout(weak.clone(), generation);
        }
    });
}

/// Hides credentials immediately and retires any scheduled reveal timeout.
pub(crate) fn hide(window: &AppWindow) {
    window.set_provider_secret_visible(false);
    window.set_provider_secret_reveal_generation(next_generation(
        window.get_provider_secret_reveal_generation(),
    ));
}

fn hide_after_timeout(weak: slint::Weak<AppWindow>, generation: i32) {
    slint::Timer::single_shot(REVEAL_DURATION, move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_provider_secret_reveal_generation() == generation {
            window.set_provider_secret_visible(false);
        }
    });
}

fn next_generation(current: i32) -> i32 {
    current.wrapping_add(1)
}

#[cfg(test)]
mod tests {
    use super::next_generation;

    #[test]
    fn each_toggle_retires_an_older_timeout() {
        assert_eq!(next_generation(12), 13);
        assert_eq!(next_generation(i32::MAX), i32::MIN);
    }
}
