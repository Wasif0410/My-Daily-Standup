//! Tests for the prompt template layer.
//!
//! The headline case is the last one: a task tree far larger than the budget
//! still has to produce a prompt that fits the smallest hardware profile's
//! context window. Everything above it guards the substitution, because a
//! template bug reaches the user as a bad answer rather than as an error.

use chrono::{NaiveDate, Weekday};

use super::context::{
    build_context, estimate_tokens, SessionContext, SessionKind, TokenBudget, RESERVED_FOR_REPLY,
    TEMPLATE_OVERHEAD,
};
use super::prompt::*;
use crate::storage::{Db, NewTask, TaskHorizon, TaskRepo, TaskSource};

/// The Lightweight profile's context window (spec, Hardware Profiles).
const LIGHTWEIGHT_CONTEXT: usize = 2_048;

fn setup() -> (Db, TaskRepo) {
    (
        Db::open_in_memory().expect("open in-memory database"),
        TaskRepo::new(),
    )
}

fn today() -> NaiveDate {
    NaiveDate::parse_from_str("2026-08-26", "%Y-%m-%d").unwrap()
}

fn context_for(db: &Db, repo: &TaskRepo, budget: TokenBudget) -> SessionContext {
    build_context(
        repo,
        db.conn(),
        SessionKind::DailyStandup,
        budget,
        today(),
        Weekday::Mon,
    )
    .expect("build the context")
}

fn empty_context() -> SessionContext {
    let (db, repo) = setup();
    context_for(&db, &repo, TokenBudget::default())
}

#[test]
fn every_known_variable_is_substituted() {
    let ctx = empty_context();

    let rendered = render_prompt(
        "{{date}} | {{session}} | {{commitment_map}} | {{ranked_context}}",
        &ctx,
    )
    .unwrap();

    assert!(rendered.contains("2026-08-26"), "{rendered}");
    assert!(rendered.contains("daily standup"), "{rendered}");
    assert!(rendered.contains("COMMITMENT MAP"), "{rendered}");
    assert!(rendered.contains("RANKED CONTEXT"), "{rendered}");
    assert!(
        !rendered.contains("{{"),
        "no placeholder may survive rendering:\n{rendered}"
    );
}

#[test]
fn whitespace_inside_a_placeholder_is_ignored() {
    let ctx = empty_context();

    let rendered = render_prompt("{{  date  }}", &ctx).unwrap();

    assert_eq!(rendered, "2026-08-26");
}

#[test]
fn an_unknown_variable_is_a_failure_rather_than_literal_text() {
    let ctx = empty_context();

    let error = render_prompt("Today is {{dat}}.", &ctx).unwrap_err();

    assert!(
        matches!(&error, PromptError::UnknownVariable { name, .. } if name == "dat"),
        "a typo must not reach the model as braces: {error:?}"
    );
    assert!(
        error.to_string().contains("date"),
        "the error must name what was available: {error}"
    );
}

#[test]
fn an_unclosed_placeholder_is_a_failure() {
    let ctx = empty_context();

    let error = render_prompt("Today is {{date.", &ctx).unwrap_err();

    assert!(
        matches!(error, PromptError::UnclosedPlaceholder { .. }),
        "{error:?}"
    );
}

#[test]
fn a_template_with_no_placeholders_renders_unchanged() {
    let ctx = empty_context();

    assert_eq!(render_prompt("plain prose", &ctx).unwrap(), "plain prose");
}

#[test]
fn the_daily_standup_template_loads_by_name() {
    let template = load_template("daily-standup").unwrap();

    assert!(template.contains("{{commitment_map}}"));
    assert!(template.contains("{{ranked_context}}"));
}

#[test]
fn an_unknown_template_name_is_a_failure() {
    let error = load_template("evening-review").unwrap_err();

    assert!(
        matches!(error, PromptError::UnknownTemplate { .. }),
        "{error:?}"
    );
}

#[test]
fn every_builtin_template_renders_with_the_variables_that_exist() {
    let ctx = empty_context();

    for (name, body) in BUILTIN_TEMPLATES {
        render_prompt(body, &ctx)
            .unwrap_or_else(|error| panic!("the {name} template does not render: {error}"));
    }
}

#[test]
fn builtin_templates_fit_the_overhead_allowance() {
    for (name, body) in BUILTIN_TEMPLATES {
        let cost = estimate_tokens(&static_text(body));

        assert!(
            cost <= TEMPLATE_OVERHEAD,
            "the {name} template's own prose costs {cost} tokens against an allowance of \
             {TEMPLATE_OVERHEAD}; growing it silently steals the map's budget"
        );
    }
}

#[test]
fn a_task_tree_far_larger_than_the_budget_still_fits_the_lightweight_context() {
    let (db, repo) = setup();

    // Twelve hundred tasks across sixty groups: far more than any budget, and
    // more than a real user would reach in a year.
    for section in 0..60 {
        let area = format!("Life area {section}");

        let mut monthly = NewTask::new(
            format!("commitment {section} for the month ahead"),
            TaskHorizon::Monthly,
            TaskSource::Manual,
        );
        monthly.area = Some(area.clone());
        monthly.priority = Some(section as i64 % 10);
        monthly.progress_current = Some(3.0);
        monthly.progress_target = Some(10.0);
        repo.create(db.conn(), monthly).unwrap();

        for child in 0..19 {
            let mut weekly = NewTask::new(
                format!("a milestone with a fairly long descriptive title {section}-{child}"),
                TaskHorizon::Weekly,
                TaskSource::Manual,
            );
            weekly.area = Some(area.clone());
            weekly.due_date = Some("2026-08-28".to_string());
            weekly.notes = Some("some notes that must never reach the prompt".to_string());
            repo.create(db.conn(), weekly).unwrap();
        }
    }

    let budget = TokenBudget::for_context_size(LIGHTWEIGHT_CONTEXT);
    let ctx = context_for(&db, &repo, budget);
    let prompt = render_session(&ctx).unwrap();

    let cost = estimate_tokens(&prompt);
    let ceiling = LIGHTWEIGHT_CONTEXT - RESERVED_FOR_REPLY;

    assert!(
        cost <= ceiling,
        "the prompt cost {cost} tokens against a {ceiling}-token ceiling on the \
         Lightweight profile — a prompt this size is silently truncated from the front"
    );
    assert!(
        prompt.contains("COMMITMENT MAP"),
        "fitting the budget must not mean dropping the map"
    );
    assert!(
        prompt.contains("incomplete"),
        "a prompt this heavily cut must say so:\n{prompt}"
    );
}

#[test]
fn the_rendered_prompt_reports_the_tiers_it_actually_carries() {
    let ctx = empty_context();

    let prompt = render_session(&ctx).unwrap();

    assert!(estimate_tokens(&prompt) >= ctx.total_tokens());
}
