use super::*;

#[test]
fn every_direction_requires_all_isolated_and_conflicting_choices() {
    for direction in [Direction::Upload, Direction::Download, Direction::BothWays] {
        let mut selection = selection(direction);
        assert!(selection.selected().is_err());
        let (local, remote, conflict) = if direction == Direction::Download {
            (
                MirrorChoice::Delete,
                MirrorChoice::Keep,
                MirrorChoice::UseRemote,
            )
        } else {
            (
                MirrorChoice::Keep,
                MirrorChoice::Delete,
                MirrorChoice::UseLocal,
            )
        };
        selection.choose(&path("local"), Some(local));
        assert!(selection.selected().is_err());
        selection.choose(&path("remote"), Some(remote));
        assert!(selection.selected().is_err());
        selection.choose(&path("conflict"), Some(conflict));
        assert!(selection.selected().is_ok());
        selection.choose(&path("remote"), None);
        assert!(selection.selected().is_err());
        selection.choose(&path("remote"), Some(remote));
        assert!(selection.selected().is_ok());
    }
}

#[test]
fn automatic_additive_skips_and_in_sync_files_need_no_manual_choice() {
    for direction in [Direction::Upload, Direction::Download, Direction::BothWays] {
        // Rebuild the same comparison in Add Only mode to exercise default choices.
        let local = inventory(&[("local", 1), ("conflict", 2), ("same", 3)]);
        let remote = inventory(&[("remote", 1), ("conflict", 4), ("same", 3)]);
        let (source, destination) = if direction == Direction::Download {
            (&remote, &local)
        } else {
            (&local, &remote)
        };
        let original = preflight(
            SyncMode::AddOnly,
            direction,
            source,
            CaseSensitivity::Sensitive,
            destination,
            CaseSensitivity::Sensitive,
        )
        .unwrap();
        let selection = MirrorSelection::new(original);
        assert!(selection.selected().is_ok());
    }
}

#[test]
fn fully_in_sync_review_can_still_be_accepted() {
    let files = inventory(&[("same", 3)]);
    let original = preflight(
        SyncMode::Mirror,
        Direction::BothWays,
        &files,
        CaseSensitivity::Sensitive,
        &files,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    assert!(MirrorSelection::new(original).selected().is_ok());
}
