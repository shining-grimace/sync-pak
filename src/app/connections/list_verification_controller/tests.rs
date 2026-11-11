use super::{SessionVerifications, VerificationState, VerificationStates, status};

#[test]
fn hides_past_verification_but_keeps_actionable_and_session_statuses() {
    let states: VerificationStates = Default::default();
    let sessions: SessionVerifications = Default::default();
    assert_eq!(status(&states, &sessions, "connection", true), "");
    assert_eq!(
        status(&states, &sessions, "connection", false),
        "Not verified"
    );

    sessions.borrow_mut().insert("connection".into());
    assert_eq!(
        status(&states, &sessions, "connection", true),
        "Verified this session"
    );
    states
        .borrow_mut()
        .insert("connection".into(), VerificationState::Checking);
    assert_eq!(status(&states, &sessions, "connection", true), "Checking");
}
