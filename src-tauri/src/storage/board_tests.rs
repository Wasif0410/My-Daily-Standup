//! Tests for board window persistence.
//!
//! Spec §23 requires that boards restore their positions after restarting.
//! These cover the storage half of that; the window creation itself needs a
//! running Tauri runtime and is verified by launching the app.

use super::*;

fn setup() -> (Db, BoardRepo) {
    (
        Db::open_in_memory().expect("open in-memory database"),
        BoardRepo::new(),
    )
}

#[test]
fn an_unsaved_board_reports_its_defaults() {
    let (db, repo) = setup();

    let board = repo.get(db.conn(), BoardKind::Priority).unwrap();

    assert_eq!(board.kind, BoardKind::Priority);
    assert_eq!(
        board.x, None,
        "position is unset until the window is placed"
    );
    assert!(!board.visible, "boards start hidden until opened");
    assert_eq!(board.opacity, 1.0);
}

#[test]
fn geometry_round_trips() {
    let (db, repo) = setup();

    let saved = BoardWindow {
        kind: BoardKind::WeeklyTasks,
        x: Some(1200),
        y: Some(340),
        width: 420,
        height: 600,
        monitor: Some(r"\\.\DISPLAY2".into()),
        visible: true,
        collapsed: true,
        opacity: 0.85,
        always_on_top: true,
        locked: true,
    };
    repo.save(db.conn(), &saved).unwrap();

    let loaded = repo.get(db.conn(), BoardKind::WeeklyTasks).unwrap();

    assert_eq!(loaded, saved);
}

#[test]
fn saving_twice_updates_rather_than_duplicating() {
    // The board's identity is its kind, so a move must overwrite the row, not
    // add a second one.
    let (db, repo) = setup();

    let mut board = BoardWindow::default_for(BoardKind::Priority);
    board.x = Some(100);
    repo.save(db.conn(), &board).unwrap();

    board.x = Some(500);
    repo.save(db.conn(), &board).unwrap();

    let count: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM board_windows", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        repo.get(db.conn(), BoardKind::Priority).unwrap().x,
        Some(500)
    );
}

#[test]
fn a_negative_position_is_preserved() {
    // A second monitor left of the primary has negative coordinates. Clamping
    // them to zero would yank the board onto the wrong screen.
    let (db, repo) = setup();

    let mut board = BoardWindow::default_for(BoardKind::MonthlyProgress);
    board.x = Some(-1720);
    board.y = Some(-200);
    repo.save(db.conn(), &board).unwrap();

    let loaded = repo.get(db.conn(), BoardKind::MonthlyProgress).unwrap();
    assert_eq!(loaded.x, Some(-1720));
    assert_eq!(loaded.y, Some(-200));
}

#[test]
fn all_returns_every_board_even_when_none_are_saved() {
    let (db, repo) = setup();

    let boards = repo.all(db.conn()).unwrap();

    assert_eq!(boards.len(), BoardKind::ALL.len());
}

#[test]
fn all_mixes_saved_state_with_defaults() {
    let (db, repo) = setup();

    let mut saved = BoardWindow::default_for(BoardKind::Priority);
    saved.visible = true;
    saved.x = Some(42);
    repo.save(db.conn(), &saved).unwrap();

    let boards = repo.all(db.conn()).unwrap();
    let priority = boards
        .iter()
        .find(|b| b.kind == BoardKind::Priority)
        .unwrap();
    let untouched = boards
        .iter()
        .find(|b| b.kind == BoardKind::WeeklyTasks)
        .unwrap();

    assert_eq!(priority.x, Some(42));
    assert_eq!(untouched.x, None);
}

#[test]
fn a_fully_transparent_board_is_rejected() {
    // Opacity 0 would make a board invisible and unclickable, with no way to
    // recover it. The schema floors it at 0.2.
    let (db, repo) = setup();

    let mut board = BoardWindow::default_for(BoardKind::Priority);
    board.opacity = 0.0;

    assert!(repo.save(db.conn(), &board).is_err());
}

#[test]
fn an_unusably_small_board_is_rejected() {
    let (db, repo) = setup();

    let mut board = BoardWindow::default_for(BoardKind::Priority);
    board.width = 10;

    assert!(repo.save(db.conn(), &board).is_err());
}

#[test]
fn every_board_kind_survives_a_round_trip() {
    // The enum's strings must match the schema's CHECK constraint; a mismatch
    // only shows up at runtime.
    let (db, repo) = setup();

    for &kind in BoardKind::ALL {
        let board = BoardWindow::default_for(kind);
        repo.save(db.conn(), &board)
            .unwrap_or_else(|e| panic!("save failed for {kind:?}: {e}"));

        assert_eq!(repo.get(db.conn(), kind).unwrap().kind, kind);
    }
}

#[test]
fn window_labels_are_unique_per_board() {
    // Tauri identifies windows by label; a collision would silently reuse one
    // window for two boards.
    let mut labels: Vec<String> = BoardKind::ALL.iter().map(|k| k.window_label()).collect();
    labels.sort();
    let total = labels.len();
    labels.dedup();

    assert_eq!(labels.len(), total, "board window labels must be unique");
}
