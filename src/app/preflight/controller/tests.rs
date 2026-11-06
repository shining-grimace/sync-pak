use super::{is_current_generation, matches_filter, next_generation};

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

#[test]
fn planned_rows_are_categorised_by_the_selected_filter() {
    assert!(matches_filter("New", 1));
    assert!(matches_filter("Will overwrite", 2));
    assert!(matches_filter("Will delete", 3));
    assert!(matches_filter("Warning", 4));
    assert!(matches_filter("Unchanged", 0));
    assert!(!matches_filter("Warning", 3));
    assert!(!matches_filter("New", 8));
}
