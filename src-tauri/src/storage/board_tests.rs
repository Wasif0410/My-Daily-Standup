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
        font_size: 15.0,
        theme: BoardTheme::Light,
        compact: true,
        desktop_level: false,
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

// ---- appearance (spec §6.7) ---------------------------------------------------

#[test]
fn a_new_board_defaults_to_dark_at_thirteen_pixels() {
    let (db, repo) = setup();

    let board = repo.get(db.conn(), BoardKind::Priority).unwrap();

    assert_eq!(board.font_size, 13.0);
    assert_eq!(board.theme, BoardTheme::Dark);
    assert!(!board.compact);
    assert!(!board.desktop_level);
}

#[test]
fn appearance_round_trips() {
    let (db, repo) = setup();
    let mut board = BoardWindow::default_for(BoardKind::WeeklyTasks);
    board.font_size = 18.0;
    board.theme = BoardTheme::Light;
    board.compact = true;
    board.desktop_level = true;

    repo.save(db.conn(), &board).unwrap();
    let loaded = repo.get(db.conn(), BoardKind::WeeklyTasks).unwrap();

    assert_eq!(loaded.font_size, 18.0);
    assert_eq!(loaded.theme, BoardTheme::Light);
    assert!(loaded.compact);
    assert!(loaded.desktop_level);
}

#[test]
fn a_board_saved_before_this_migration_gains_the_defaults() {
    // The migration adds columns to a table that may already hold rows. Without
    // a DEFAULT on each, an existing board would fail to load rather than
    // simply looking the way it always did.
    let (db, repo) = setup();
    db.conn()
        .execute(
            "INSERT INTO board_windows
                 (kind, width, height, visible, collapsed, opacity,
                  always_on_top, locked, updated_at)
             VALUES ('priority', 340, 460, 1, 0, 1.0, 0, 0, '2026-08-01T00:00:00Z')",
            [],
        )
        .unwrap();

    let board = repo.get(db.conn(), BoardKind::Priority).unwrap();

    assert_eq!(board.font_size, 13.0);
    assert_eq!(board.theme, BoardTheme::Dark);
}

#[test]
fn a_font_size_below_the_floor_is_rejected() {
    // At 8px the board is one illegible smudge and the menu that would undo it
    // is unreadable too.
    let (db, repo) = setup();
    let mut board = BoardWindow::default_for(BoardKind::Priority);
    board.font_size = 8.0;

    assert!(repo.save(db.conn(), &board).is_err());
}

#[test]
fn a_font_size_above_the_ceiling_is_rejected() {
    let (db, repo) = setup();
    let mut board = BoardWindow::default_for(BoardKind::Priority);
    board.font_size = 40.0;

    assert!(repo.save(db.conn(), &board).is_err());
}

#[test]
fn the_opacity_floor_is_still_enforced() {
    // Unchanged by this migration, but the reason is the same as the font
    // clamp: a fully transparent board is invisible and unclickable.
    let (db, repo) = setup();
    let mut board = BoardWindow::default_for(BoardKind::Priority);
    board.opacity = 0.0;

    assert!(repo.save(db.conn(), &board).is_err());
}

#[test]
fn an_unknown_theme_is_rejected() {
    // Written as raw SQL because the repository cannot express an invalid
    // theme — `BoardTheme` has no third variant. The CHECK is what guards the
    // file against a hand-edit or a future migration bug.
    let (db, _repo) = setup();

    let written = db.conn().execute(
        "INSERT INTO board_windows
             (kind, width, height, visible, collapsed, opacity, always_on_top,
              locked, theme, updated_at)
         VALUES ('priority', 340, 460, 1, 0, 1.0, 0, 0, 'solarized', 'now')",
        [],
    );

    assert!(written.is_err());
}

#[test]
fn a_window_label_identifies_its_board() {
    // The window event handler only receives a label. Without this, a board
    // closed by Alt+F4 cannot be told from any other window, and the close
    // cannot be routed to the hide-and-record path.
    for &kind in BoardKind::ALL {
        assert_eq!(
            BoardKind::from_window_label(&kind.window_label()),
            Some(kind),
            "{kind:?} must be recoverable from its own label"
        );
    }
}

#[test]
fn only_board_labels_name_a_board() {
    // The main window and the capture box are not boards, and a bare kind is
    // not a label. Treating any of them as one would hide the wrong window.
    assert_eq!(BoardKind::from_window_label("main"), None);
    assert_eq!(BoardKind::from_window_label("quick-add"), None);
    assert_eq!(BoardKind::from_window_label("priority"), None);
    assert_eq!(BoardKind::from_window_label("board-nonsense"), None);
    assert_eq!(BoardKind::from_window_label(""), None);
}
