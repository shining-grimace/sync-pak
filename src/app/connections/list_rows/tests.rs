use super::*;

fn row(id: &str, result_id: &str) -> ConnectionRow {
    ConnectionRow {
        id: id.into(),
        result_id: result_id.into(),
        ..Default::default()
    }
}

#[test]
fn verification_refreshes_keep_summaries_and_the_existing_model() {
    let model = update(
        ModelRc::default(),
        vec![row("a", "summary-a"), row("b", "summary-b")],
    );
    let mut checking = row("b", "");
    checking.verifying = true;
    checking.verification = "Checking".into();
    let refreshed = update(model.clone(), vec![row("a", ""), checking]);
    assert_eq!(refreshed, model);
    assert_eq!(refreshed.row_data(0).unwrap().result_id, "summary-a");
    let verified = refreshed.row_data(1).unwrap();
    assert_eq!(verified.result_id, "summary-b");
    assert!(verified.verifying);
    let finished = update(refreshed, vec![row("a", ""), row("b", "")]);
    assert_eq!(finished.row_data(1).unwrap().result_id, "summary-b");
    assert!(!finished.row_data(1).unwrap().verifying);
}

#[test]
fn reordering_keeps_summaries_with_their_connection() {
    let model = update(
        ModelRc::default(),
        vec![row("a", "summary-a"), row("b", "summary-b")],
    );
    let refreshed = update(model, vec![row("b", ""), row("new", ""), row("a", "")]);
    assert_eq!(refreshed.row_data(0).unwrap().result_id, "summary-b");
    assert_eq!(refreshed.row_data(1).unwrap().result_id, "");
    assert_eq!(refreshed.row_data(2).unwrap().result_id, "summary-a");
}

#[test]
fn an_activity_refresh_can_remove_a_summary_without_it_reappearing() {
    let model = update(ModelRc::default(), vec![row("a", "summary-a")]);
    model.set_row_data(0, row("a", ""));
    assert_eq!(
        update(model, vec![row("a", "")])
            .row_data(0)
            .unwrap()
            .result_id,
        ""
    );
}
