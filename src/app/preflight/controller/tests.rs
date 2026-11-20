use super::{is_current_generation, next_generation};

#[test]
fn only_the_current_preflight_attempt_can_publish_a_result() {
    assert!(is_current_generation(4, 4));
    assert!(!is_current_generation(4, 5));
}

#[test]
fn generation_advances_across_integer_wraparound() {
    assert_eq!(next_generation(4), 5);
    assert_eq!(next_generation(i32::MAX), i32::MIN);
}
