//! Tests for the ranked context and the on-demand subtree.
//!
//! Tier 2's job is to say what is urgent and *why*. Most of these tests are
//! about the "why": a line that reads "this has moved four times and is
//! blocked on a reply" is worth its tokens, and one that reads "this is on
//! your list" is not.

use chrono::{NaiveDate, Weekday};

use super::context::*;
use crate::storage::{
    Db, NewTask, StorageError, Task, TaskHorizon, TaskPatch, TaskRepo, TaskSource, TaskStatus,
};

const TODAY: &str = "2026-08-26";

fn today() -> NaiveDate {
    NaiveDate::parse_from_str(TODAY, "%Y-%m-%d").unwrap()
}

fn setup() -> (Db, TaskRepo) {
    (
        Db::open_in_memory().expect("open in-memory database"),
        TaskRepo::new(),
    )
}

fn task(db: &Db, title: &str, horizon: TaskHorizon, edit: impl FnOnce(&mut NewTask)) -> Task {
    let mut input = NewTask::new(title, horizon, TaskSource::Manual);
    edit(&mut input);
    TaskRepo::new()
        .create(db.conn(), input)
        .expect("create task")
}

fn context(db: &Db, repo: &TaskRepo) -> SessionContext {
    build_context(
        repo,
        db.conn(),
        SessionKind::DailyStandup,
        TokenBudget::default(),
        today(),
        Weekday::Mon,
    )
    .expect("build the context")
}

fn titles(ctx: &SessionContext) -> Vec<&str> {
    ctx.items.iter().map(|item| item.title.as_str()).collect()
}

#[test]
fn this_months_commitment_is_in_the_ranked_context() {
    let (db, repo) = setup();
    task(&db, "applications this month", TaskHorizon::Monthly, |i| {
        i.period_start = Some("2026-08-01".to_string());
        i.period_end = Some("2026-08-31".to_string());
    });

    let ctx = context(&db, &repo);

    assert_eq!(titles(&ctx), vec!["applications this month"]);
    assert_eq!(ctx.items[0].group, ContextGroup::MonthlyCommitment);
}

#[test]
fn this_weeks_milestone_is_in_the_ranked_context() {
    let (db, repo) = setup();
    task(&db, "Rewrite resume", TaskHorizon::Weekly, |i| {
        i.period_start = Some("2026-08-24".to_string());
        i.period_end = Some("2026-08-30".to_string());
    });

    let ctx = context(&db, &repo);

    assert_eq!(ctx.items[0].group, ContextGroup::WeeklyCommitment);
}

#[test]
fn a_commitment_from_another_month_is_left_out() {
    let (db, repo) = setup();
    task(&db, "last month's thing", TaskHorizon::Monthly, |i| {
        i.period_start = Some("2026-06-01".to_string());
        i.period_end = Some("2026-06-30".to_string());
    });

    let ctx = context(&db, &repo);

    assert!(
        titles(&ctx).is_empty(),
        "a commitment outside the month is not this standup's business: {:#?}",
        ctx.items
    );
}

#[test]
fn work_scheduled_before_today_and_still_open_is_reported_as_unfinished() {
    let (db, repo) = setup();
    task(&db, "yesterday's call", TaskHorizon::Daily, |i| {
        i.scheduled_date = Some("2026-08-25".to_string());
    });

    let ctx = context(&db, &repo);

    assert_eq!(ctx.items[0].group, ContextGroup::Unfinished);
}

#[test]
fn an_upcoming_due_date_is_reported() {
    let (db, repo) = setup();
    task(&db, "file the form", TaskHorizon::Daily, |i| {
        i.due_date = Some("2026-08-28".to_string());
    });

    let ctx = context(&db, &repo);

    assert_eq!(ctx.items[0].group, ContextGroup::DueSoon);
    assert!(
        ctx.items[0].detail.iter().any(|d| d.contains("2026-08-28")),
        "a due date item that does not say when is worth nothing: {:#?}",
        ctx.items[0]
    );
}

#[test]
fn a_due_date_far_out_is_not_yet_this_standups_business() {
    let (db, repo) = setup();
    task(&db, "next month's form", TaskHorizon::Daily, |i| {
        i.due_date = Some("2026-11-01".to_string());
    });

    let ctx = context(&db, &repo);

    assert!(titles(&ctx).is_empty(), "{:#?}", ctx.items);
}

#[test]
fn a_repeatedly_deferred_task_is_raised_with_its_count() {
    let (db, repo) = setup();
    let created = task(&db, "Schedule dental cleaning", TaskHorizon::Daily, |i| {
        i.scheduled_date = Some("2026-09-01".to_string());
    });
    for day in ["2026-09-02", "2026-09-03", "2026-09-04"] {
        crate::domain::reschedule(&repo, db.conn(), &created.id, day).unwrap();
    }

    let ctx = context(&db, &repo);

    assert_eq!(ctx.items[0].group, ContextGroup::RepeatedlyDeferred);
    assert!(
        ctx.items[0].detail.iter().any(|d| d == "moved 3×"),
        "the count is the point of §11.2's prompt: {:#?}",
        ctx.items[0]
    );
}

#[test]
fn a_task_moved_once_is_not_yet_a_pattern() {
    let (db, repo) = setup();
    let created = task(&db, "a normal slip", TaskHorizon::Daily, |i| {
        i.scheduled_date = Some("2026-09-01".to_string());
    });
    crate::domain::reschedule(&repo, db.conn(), &created.id, "2026-09-02").unwrap();

    let ctx = context(&db, &repo);

    assert!(titles(&ctx).is_empty(), "{:#?}", ctx.items);
}

#[test]
fn an_item_carries_the_detail_that_makes_it_meaningful() {
    let (db, repo) = setup();
    let created = task(&db, "applications this month", TaskHorizon::Monthly, |i| {
        i.priority = Some(9);
        i.progress_current = Some(12.0);
        i.progress_target = Some(20.0);
        i.progress_unit = Some("applications".to_string());
    });
    crate::domain::set_blocker(&repo, db.conn(), &created.id, Some("waiting on a referral"))
        .unwrap();
    repo.update(
        db.conn(),
        &created.id,
        TaskPatch {
            time_spent_minutes: Some(Some(90)),
            ..Default::default()
        },
    )
    .unwrap();

    let ctx = context(&db, &repo);
    let rendered = ctx.items[0].render();

    assert!(
        rendered.contains("blocked: waiting on a referral"),
        "{rendered}"
    );
    assert!(rendered.contains("12/20 applications"), "{rendered}");
    assert!(rendered.contains("p9"), "{rendered}");
    assert!(rendered.contains("1h 30m logged"), "{rendered}");
}

#[test]
fn a_task_that_qualifies_twice_is_listed_once_under_its_higher_group() {
    let (db, repo) = setup();
    task(&db, "overdue commitment", TaskHorizon::Monthly, |i| {
        i.period_start = Some("2026-08-01".to_string());
        i.period_end = Some("2026-08-31".to_string());
        i.scheduled_date = Some("2026-08-20".to_string());
        i.due_date = Some("2026-08-27".to_string());
    });

    let ctx = context(&db, &repo);

    assert_eq!(ctx.items.len(), 1, "{:#?}", ctx.items);
    assert_eq!(ctx.items[0].group, ContextGroup::MonthlyCommitment);
}

#[test]
fn completed_work_is_not_ranked_context() {
    let (db, repo) = setup();
    task(&db, "already done", TaskHorizon::Monthly, |i| {
        i.status = TaskStatus::Completed;
    });

    let ctx = context(&db, &repo);

    assert!(titles(&ctx).is_empty(), "{:#?}", ctx.items);
}

#[test]
fn the_ranked_context_never_exceeds_its_token_budget() {
    let (db, repo) = setup();
    for index in 0..400 {
        task(
            &db,
            &format!("commitment number {index}"),
            TaskHorizon::Monthly,
            |i| {
                i.priority = Some(5);
            },
        );
    }

    let budget = TokenBudget {
        map: 1_500,
        context: 300,
    };
    let ctx = build_context(
        &repo,
        db.conn(),
        SessionKind::DailyStandup,
        budget,
        today(),
        Weekday::Mon,
    )
    .unwrap();

    assert!(
        ctx.context_tokens <= budget.context,
        "tier 2 spent {} of a {}-token budget",
        ctx.context_tokens,
        budget.context
    );
    assert!(ctx.truncated);
}

#[test]
fn the_ranked_context_never_exceeds_its_item_cap() {
    let (db, repo) = setup();
    for index in 0..MAX_CONTEXT_ITEMS * 3 {
        task(
            &db,
            &format!("commitment {index}"),
            TaskHorizon::Monthly,
            |i| {
                i.priority = Some(5);
            },
        );
    }

    let ctx = context(&db, &repo);

    assert_eq!(
        ctx.items.len(),
        MAX_CONTEXT_ITEMS,
        "the count cap holds even with budget to spare"
    );
    assert!(ctx.truncated);
}

#[test]
fn overflow_drops_the_lowest_ranked_items_first() {
    let (db, repo) = setup();
    task(&db, "the commitment", TaskHorizon::Monthly, |i| {
        i.priority = Some(9);
    });
    for index in 0..40 {
        task(
            &db,
            &format!("a mere due date {index}"),
            TaskHorizon::Daily,
            |i| {
                i.due_date = Some("2026-08-28".to_string());
            },
        );
    }

    let ctx = build_context(
        &repo,
        db.conn(),
        SessionKind::DailyStandup,
        TokenBudget {
            map: 1_500,
            context: 120,
        },
        today(),
        Weekday::Mon,
    )
    .unwrap();

    assert_eq!(
        ctx.items[0].title, "the commitment",
        "the commitment outranks every due date: {:#?}",
        ctx.items
    );
    assert!(ctx.truncated);
}

#[test]
fn a_truncated_context_tells_the_model_it_is_incomplete() {
    let (db, repo) = setup();
    for index in 0..60 {
        task(
            &db,
            &format!("commitment {index}"),
            TaskHorizon::Monthly,
            |i| {
                i.priority = Some(5);
            },
        );
    }

    let ctx = build_context(
        &repo,
        db.conn(),
        SessionKind::DailyStandup,
        TokenBudget {
            map: 1_500,
            context: 200,
        },
        today(),
        Weekday::Mon,
    )
    .unwrap();

    assert!(
        ctx.render_context().contains("incomplete"),
        "a list that merely stops reads as a complete one:\n{}",
        ctx.render_context()
    );
}

#[test]
fn an_empty_board_set_produces_a_context_that_says_nothing_is_outstanding() {
    let (db, repo) = setup();

    let ctx = context(&db, &repo);

    assert!(ctx.render_context().contains("nothing outstanding"));
    assert!(!ctx.truncated);
}

#[test]
fn a_lightweight_context_window_shrinks_both_budgets_to_fit() {
    let budget = TokenBudget::for_context_size(2_048);

    assert!(
        budget.map > 0,
        "a session without a map is a session about nothing"
    );
    assert!(budget.context > 0);
    assert!(
        budget.total() + RESERVED_FOR_REPLY + TEMPLATE_OVERHEAD <= 2_048,
        "the budget must leave room for the template and the reply: {budget:?}"
    );
}

#[test]
fn a_generous_context_window_keeps_the_specified_budgets() {
    let budget = TokenBudget::for_context_size(8_192);

    assert_eq!(budget.map, DEFAULT_MAP_BUDGET);
    assert_eq!(budget.context, DEFAULT_CONTEXT_BUDGET);
}

#[test]
fn fetch_task_subtree_returns_the_children_notes_and_blocker() {
    let (db, repo) = setup();
    let parent = task(&db, "applications this month", TaskHorizon::Monthly, |i| {
        i.notes = Some("aim for five a week".to_string());
    });
    task(&db, "Fall 2026 applications", TaskHorizon::Weekly, |i| {
        i.parent_task_id = Some(parent.id.clone());
    });
    crate::domain::set_blocker(&repo, db.conn(), &parent.id, Some("waiting on a referral"))
        .unwrap();

    let subtree = fetch_task_subtree(&repo, db.conn(), &parent.id).unwrap();

    assert_eq!(subtree.task.id, parent.id);
    assert_eq!(subtree.children.len(), 1);
    assert_eq!(subtree.children[0].title, "Fall 2026 applications");
    assert_eq!(subtree.notes.as_deref(), Some("aim for five a week"));
    assert_eq!(subtree.blocker.as_deref(), Some("waiting on a referral"));
}

#[test]
fn fetch_task_subtree_on_a_leaf_returns_no_children() {
    let (db, repo) = setup();
    let leaf = task(&db, "Book eye test", TaskHorizon::Daily, |_| {});

    let subtree = fetch_task_subtree(&repo, db.conn(), &leaf.id).unwrap();

    assert!(subtree.children.is_empty(), "a leaf has no children");
    assert!(subtree.ancestors.is_empty());
    assert_eq!(subtree.notes, None);
    assert_eq!(subtree.blocker, None);
}

#[test]
fn fetch_task_subtree_carries_the_commitment_a_milestone_sits_under() {
    let (db, repo) = setup();
    let parent = task(&db, "applications this month", TaskHorizon::Monthly, |_| {});
    let child = task(&db, "Fall 2026 applications", TaskHorizon::Weekly, |i| {
        i.parent_task_id = Some(parent.id.clone());
    });

    let subtree = fetch_task_subtree(&repo, db.conn(), &child.id).unwrap();

    assert_eq!(subtree.ancestors.len(), 1);
    assert_eq!(subtree.ancestors[0].id, parent.id);
}

#[test]
fn fetch_task_subtree_refuses_an_id_that_resolves_to_nothing() {
    let (db, repo) = setup();

    let error = fetch_task_subtree(&repo, db.conn(), "not-a-real-id").unwrap_err();

    assert!(
        matches!(error, StorageError::TaskNotFound { .. }),
        "validating model output depends on this being an error: {error:?}"
    );
}

#[test]
fn fetch_task_subtree_stops_walking_a_deep_parent_chain() {
    let (db, repo) = setup();

    let mut parent: Option<String> = None;
    let mut deepest = String::new();
    for index in 0..500 {
        let created = task(&db, &format!("link {index}"), TaskHorizon::Daily, |input| {
            input.parent_task_id = parent.clone();
        });
        parent = Some(created.id.clone());
        deepest = created.id;
    }

    let subtree = fetch_task_subtree(&repo, db.conn(), &deepest).unwrap();

    assert_eq!(
        subtree.ancestors.len(),
        MAX_ANCESTOR_DEPTH,
        "a five-hundred-deep chain must stop at the cap rather than climb it"
    );
}

#[test]
fn fetch_task_subtree_does_not_hang_on_a_cyclic_parent_chain() {
    let (db, repo) = setup();
    let first = task(&db, "first", TaskHorizon::Daily, |_| {});
    let second = task(&db, "second", TaskHorizon::Daily, |i| {
        i.parent_task_id = Some(first.id.clone());
    });
    repo.update(
        db.conn(),
        &first.id,
        TaskPatch {
            parent_task_id: Some(Some(second.id.clone())),
            ..Default::default()
        },
    )
    .unwrap();

    let subtree = fetch_task_subtree(&repo, db.conn(), &second.id).unwrap();

    assert!(
        subtree.ancestors.len() <= MAX_ANCESTOR_DEPTH,
        "a cycle must terminate, not spin"
    );
}

#[test]
fn a_needs_context_reply_is_recognised_and_its_id_extracted() {
    // The prompt tells the model it may answer NEEDS_CONTEXT <id> to see one
    // task in full. Until something reads that, the model obeys and the raw
    // protocol line lands in the user's chat window looking like a crash.
    let id = needs_context_id("NEEDS_CONTEXT [5175371a-63fd-4b72-8d8e-fa5afb1861d6]");

    assert_eq!(id.as_deref(), Some("5175371a-63fd-4b72-8d8e-fa5afb1861d6"));
}

#[test]
fn the_brackets_are_optional_because_the_model_is_inconsistent_about_them() {
    // Observed both ways from the same model on the same prompt. Accepting one
    // shape only would make the feature work intermittently, which is worse
    // than not working.
    assert_eq!(
        needs_context_id("NEEDS_CONTEXT 5175371a-63fd-4b72-8d8e-fa5afb1861d6").as_deref(),
        Some("5175371a-63fd-4b72-8d8e-fa5afb1861d6")
    );
}

#[test]
fn an_ordinary_answer_is_not_mistaken_for_a_request() {
    // The guard that matters: a reply that merely mentions the phrase must not
    // trigger a fetch, or an answer about context would vanish into a hop.
    assert!(needs_context_id("You should tailor the resume today.").is_none());
    assert!(
        needs_context_id("I would need more context about your week.").is_none(),
        "prose about needing context is not the protocol"
    );
}

#[test]
fn a_request_naming_nothing_usable_is_declined_rather_than_guessed() {
    assert!(needs_context_id("NEEDS_CONTEXT").is_none());
    assert!(needs_context_id("NEEDS_CONTEXT []").is_none());
}
