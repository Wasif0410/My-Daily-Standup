//! Tier 1 — the commitment map.
//!
//! The map tells the model *what exists*; the ranker in [`super::context`]
//! tells it *what is urgent*. Keeping those two jobs apart is not tidiness:
//! collapsing them would force the model to infer priority from the shape of
//! a tree, which is exactly the judgment spec §3.6 requires to stay
//! deterministic and Rust-owned.
//!
//! Three levels, rendered as an indented tree:
//!
//! ```text
//! COMMITMENT MAP — active
//!
//! Job Search [p9, in progress] — 12/20 applications this month
//!   Fall 2026 applications ....... p7, weekly, 3/5 done
//!   Rewrite resume ............... p6, weekly, blocked
//! ```
//!
//! The headings are the user's own **sections** — the named groups the boards
//! already render (see [`crate::storage::section`]). They are what make this a
//! tree rather than a flat priority-ordered dump: the user named them, so they
//! are the grouping the user already thinks in, and the model gets the shape
//! of the week for free instead of inventing categories out of task titles.
//!
//! A section's heading line carries the month's commitment for that group; the
//! weekly milestones and daily actions indent beneath it. Every line carries
//! priority and status, because a map without them would send the model back
//! to guessing.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Serialize;

use super::context::{estimate_tokens, number, truncation_notice};
use crate::storage::{
    group_key, BoardKind, SectionField, SectionRepo, StorageError, Task, TaskHorizon, TaskRepo,
    TaskStatus,
};

/// The map's first line. Says "active" because completed and cancelled work is
/// deliberately absent, and a model that assumed otherwise would report a
/// month as empty on the day it was finished.
pub const HEADER: &str = "COMMITMENT MAP — active";

/// What tasks belonging to no named group are filed under.
///
/// The same word the boards use when a section is deleted and its tasks are
/// unfiled, so the model and the screen call the same pile the same thing.
pub const UNGROUPED_HEADING: &str = "Unsorted";

/// The most headings the map will render, however much budget is left.
///
/// Spec §9.2 sizes the tier at roughly twenty headings. The count cap exists
/// beside the token cap because a hundred one-word headings fit any budget and
/// are still a wall: a 1.7B model loses the thread of a list long before it
/// runs out of context to hold it.
pub const MAX_SECTIONS: usize = 20;

/// The most lines the map will render in total, headings included.
pub const MAX_LINES: usize = 110;

/// The most children shown under any one heading.
///
/// A per-section cap as well as a global one, so a single section with two
/// hundred tasks cannot consume the whole map and leave every other commitment
/// invisible. Losing the tail of one group is recoverable; losing every other
/// group is the failure this PR exists to prevent.
pub const MAX_MILESTONES_PER_SECTION: usize = 8;

/// The column the dot leaders run out to.
const LEADER_COLUMN: usize = 30;

/// One rendered line: a heading's commitment, or a child beneath it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapLine {
    /// The id the model quotes back to ask for this task's subtree (tier 3).
    pub id: String,
    pub title: String,
    pub horizon: TaskHorizon,
    /// Priority and status, as the bracketed pair on a heading line.
    pub priority: Option<i64>,
    pub status: TaskStatus,
    /// The tail of the line: progress, a blocker, or a deferral count.
    pub detail: String,
}

/// One heading and everything filed under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapSection {
    /// `None` for the ungrouped pile, which renders as
    /// [`UNGROUPED_HEADING`].
    pub title: Option<String>,
    /// The month's commitment for this group, if there is one.
    pub commitment: Option<MapLine>,
    pub milestones: Vec<MapLine>,
    /// What overflow drops first. See [`section_rank`].
    pub rank: i64,
    /// Set when this section's own children were cut to fit
    /// [`MAX_MILESTONES_PER_SECTION`].
    pub truncated: bool,
}

impl MapSection {
    /// The heading as it is written.
    pub fn heading(&self) -> String {
        let title = self.title.as_deref().unwrap_or(UNGROUPED_HEADING);

        let Some(commitment) = &self.commitment else {
            return title.to_string();
        };

        let mut heading = String::from(title);
        heading.push(' ');
        heading.push_str(&bracket(commitment));

        if !commitment.detail.is_empty() {
            heading.push_str(" — ");
            heading.push_str(&commitment.detail);
        }

        heading
    }

    /// The whole block, heading and children.
    pub fn render(&self) -> String {
        let mut out = self.heading();

        for milestone in &self.milestones {
            out.push('\n');
            out.push_str(&child_line(milestone));
        }

        if self.truncated {
            out.push_str("\n  ... more not shown");
        }

        out
    }

    /// Heading plus children, for the line cap.
    fn line_count(&self) -> usize {
        1 + self.milestones.len()
    }
}

/// The map, already inside its budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentMap {
    pub sections: Vec<MapSection>,
    /// Set when whole sections were dropped. The rendered text says so too —
    /// a map that merely stopped would let the model conclude it had seen
    /// everything the user is committed to, which is the one wrong answer
    /// this tier can give.
    pub truncated: bool,
    pub dropped_sections: usize,
    pub estimated_tokens: usize,
}

impl CommitmentMap {
    /// The map as the prompt sees it.
    pub fn render(&self) -> String {
        let mut out = String::from(HEADER);

        if self.sections.is_empty() {
            out.push_str("\n\n(nothing active)");
            return out;
        }

        for section in &self.sections {
            out.push_str("\n\n");
            out.push_str(&section.render());
        }

        if self.truncated {
            out.push_str("\n\n");
            out.push_str(&truncation_notice(self.dropped_sections));
        }

        out
    }

    /// Every line the map renders, headings included.
    pub fn line_count(&self) -> usize {
        self.sections.iter().map(MapSection::line_count).sum()
    }
}

/// Builds the commitment map inside `budget` estimated tokens.
///
/// Deviates from the spec's `build_map(repo, budget)` only by taking the
/// connection as well: the repository in this codebase is a stateless handle
/// and the connection is passed alongside it everywhere else.
pub fn build_map(
    repo: &TaskRepo,
    conn: &Connection,
    budget: usize,
) -> Result<CommitmentMap, StorageError> {
    let active = repo.list_active(conn)?;
    let headings = declared_headings(conn)?;

    let mut sections = assemble(&active, &headings);

    // Highest rank first, then by heading, so an overflow always drops the
    // same sections. A map whose contents shifted between runs would make
    // "why did it not mention Health" unanswerable.
    sections.sort_by(|a, b| {
        b.rank
            .cmp(&a.rank)
            .then_with(|| heading_order(a).cmp(&heading_order(b)))
    });

    let (sections, dropped) = fit_within(sections, budget);
    let truncated = dropped > 0;

    let mut map = CommitmentMap {
        sections,
        truncated,
        dropped_sections: dropped,
        estimated_tokens: 0,
    };

    map.estimated_tokens = estimate_tokens(&map.render());

    Ok(map)
}

/// The group names the user has declared, keyed the way tasks are matched.
///
/// Declared sections are read for both boards that have them — Priority groups
/// by `area`, Weekly Tasks by `project` — because a heading the user typed and
/// has not filled yet still names a real intention, and the map is the place
/// that intention is least useful hidden.
fn declared_headings(conn: &Connection) -> Result<BTreeMap<String, String>, StorageError> {
    let repo = SectionRepo::new();
    let mut headings = BTreeMap::new();

    for kind in BoardKind::ALL {
        if kind.section_field().is_none() {
            continue;
        }

        for section in repo.list(conn, *kind)? {
            headings
                .entry(group_key(&section.title))
                .or_insert(section.title);
        }
    }

    Ok(headings)
}

/// Files every active task under a heading, and the rest under none.
///
/// A task is matched to a section exactly the way the boards match it:
/// `tasks.area` for a Priority section, `tasks.project` for a Weekly Tasks
/// one, compared through [`group_key`] so "Job Search" and "job search " are
/// one group here and on screen alike.
///
/// `area` is consulted before `project`. A task carrying both names two groups
/// at once and has to land in one of them; the Priority board is the one that
/// shows work outliving a day, so its grouping is the one the map inherits.
///
/// **Tasks matching no heading are not dropped.** They are collected under
/// [`UNGROUPED_HEADING`], because the failure being avoided here is silent: a
/// user whose work vanished from the map would get a standup that never
/// mentioned it and no clue that anything was missing.
fn assemble(active: &[Task], headings: &BTreeMap<String, String>) -> Vec<MapSection> {
    // Keyed by the normalised group name; `None` is the ungrouped pile.
    let mut grouped: BTreeMap<Option<String>, Vec<&Task>> = BTreeMap::new();

    for task in active {
        grouped.entry(group_of(task)).or_default().push(task);
    }

    // A heading the user declared and has not filled still renders. It says
    // "this is a thing I am tracking and there is nothing under it", which is
    // worth a line at a standup.
    for key in headings.keys() {
        grouped.entry(Some(key.clone())).or_default();
    }

    grouped
        .into_iter()
        .map(|(key, tasks)| {
            let title = key.as_ref().map(|key| {
                headings
                    .get(key)
                    .cloned()
                    // Groups the boards derive from tasks alone, with no
                    // declared row behind them, are just as real to the user.
                    // The first task's own spelling is the name they typed.
                    .unwrap_or_else(|| displayed_name(&tasks, key))
            });

            build_section(title, &tasks)
        })
        .collect()
}

/// Which group a task belongs to, or `None` for the ungrouped pile.
fn group_of(task: &Task) -> Option<String> {
    for field in [SectionField::Area, SectionField::Project] {
        let value = match field {
            SectionField::Area => task.area.as_deref(),
            SectionField::Project => task.project.as_deref(),
        };

        if let Some(value) = value.filter(|v| !v.trim().is_empty()) {
            return Some(group_key(value));
        }
    }

    None
}

/// The spelling shown for a derived group: whatever the first task filed under
/// it actually wrote.
fn displayed_name(tasks: &[&Task], key: &str) -> String {
    tasks
        .iter()
        .find_map(|task| {
            [task.area.as_deref(), task.project.as_deref()]
                .into_iter()
                .flatten()
                .find(|value| group_key(value) == key)
                .map(|value| value.trim().to_string())
        })
        .unwrap_or_else(|| key.to_string())
}

/// Splits one group's tasks into its heading commitment and its children.
fn build_section(title: Option<String>, tasks: &[&Task]) -> MapSection {
    // The month's commitment is the heading line (spec §9.2). Where a group
    // somehow holds two, the higher-priority one leads and the other falls in
    // with the children rather than being dropped.
    let mut monthly: Vec<&&Task> = tasks
        .iter()
        .filter(|task| task.horizon == TaskHorizon::Monthly)
        .collect();
    monthly.sort_by_key(|task| std::cmp::Reverse(task.priority.unwrap_or(i64::MIN)));

    let commitment_id = monthly.first().map(|task| task.id.clone());
    let commitment = monthly.first().map(|task| heading_line(task));

    let mut children: Vec<&&Task> = tasks
        .iter()
        .filter(|task| Some(&task.id) != commitment_id.as_ref())
        .collect();

    // Weekly milestones before daily actions, then by priority. The spec's own
    // example shows both horizons under a heading, and the milestone is the
    // one that explains the day beneath it.
    children.sort_by_key(|task| {
        (
            horizon_order(task.horizon),
            std::cmp::Reverse(task.priority.unwrap_or(i64::MIN)),
            task.created_at.clone(),
        )
    });

    let truncated = children.len() > MAX_MILESTONES_PER_SECTION;
    let milestones: Vec<MapLine> = children
        .into_iter()
        .take(MAX_MILESTONES_PER_SECTION)
        .map(|task| child_of(task))
        .collect();

    let rank = section_rank(commitment.as_ref(), tasks);

    MapSection {
        title,
        commitment,
        milestones,
        rank,
        truncated,
    }
}

/// What overflow drops first.
///
/// The group's monthly commitment decides it, because that is what the user
/// said the month is for. A group with no commitment falls back to the highest
/// priority anything in it carries, so a section full of urgent work is not
/// dropped merely for lacking a heading task.
///
/// An unranked group sorts last but is still a real group. `priority IS NULL`
/// means "not judged", not "judged unimportant" — the same reading the
/// Priority board takes.
fn section_rank(commitment: Option<&MapLine>, tasks: &[&Task]) -> i64 {
    if let Some(priority) = commitment.and_then(|line| line.priority) {
        // A commitment's own priority outranks any child's, so a section led
        // by a p5 commitment does not jump the queue on the strength of one
        // p9 daily inside it.
        return priority * 10 + 5;
    }

    tasks
        .iter()
        .filter_map(|task| task.priority)
        .max()
        .map(|priority| priority * 10)
        .unwrap_or(i64::MIN)
}

/// A stable tie-break for sections of equal rank.
fn heading_order(section: &MapSection) -> (u8, String) {
    match &section.title {
        // The ungrouped pile sorts after named groups of the same rank: the
        // user named the others, and a tie should favour the name they chose.
        None => (1, String::new()),
        Some(title) => (0, group_key(title)),
    }
}

/// Weekly work leads, then daily, then anything else.
fn horizon_order(horizon: TaskHorizon) -> u8 {
    match horizon {
        TaskHorizon::Weekly => 0,
        TaskHorizon::Daily => 1,
        TaskHorizon::Monthly => 2,
        TaskHorizon::LongTerm => 3,
    }
}

/// The heading's own line: the monthly commitment.
fn heading_line(task: &Task) -> MapLine {
    let mut detail = task.title.trim().to_string();

    // "12/20 applications this month" — the figure the user set, in front of
    // the words they wrote. A commitment with a target is the one number a
    // standup is most likely to be about.
    if let Some(target) = task.progress_target {
        let current = task.progress_current.unwrap_or(0.0);
        detail = format!("{}/{} {detail}", number(current), number(target));
    }

    MapLine {
        id: task.id.clone(),
        title: task.title.trim().to_string(),
        horizon: task.horizon,
        priority: task.priority,
        status: task.status,
        detail,
    }
}

/// A child line beneath a heading.
fn child_of(task: &Task) -> MapLine {
    MapLine {
        id: task.id.clone(),
        title: task.title.trim().to_string(),
        horizon: task.horizon,
        priority: task.priority,
        status: task.status,
        detail: child_detail(task),
    }
}

/// The tail of a child line: the single most useful thing about it.
///
/// One phrase, not several — tier 2 is where a task gets its full story, and
/// repeating that here would spend the map's budget saying it twice.
///
/// Ordered by what would change the user's next move: a blocker outranks a
/// progress figure, which outranks a deferral count, which outranks the bare
/// status.
fn child_detail(task: &Task) -> String {
    if task.status == TaskStatus::Blocked
        || task
            .blocker
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
    {
        return "blocked".to_string();
    }

    if let Some(target) = task.progress_target {
        let current = task.progress_current.unwrap_or(0.0);
        return format!("{}/{} done", number(current), number(target));
    }

    if task.rollover_count > 0 {
        return format!("deferred {}×", task.rollover_count);
    }

    status_label(task.status).to_string()
}

/// The bracketed pair every heading carries.
fn bracket(line: &MapLine) -> String {
    match line.priority {
        Some(priority) => format!("[p{priority}, {}]", status_label(line.status)),
        None => format!("[{}]", status_label(line.status)),
    }
}

/// A child line, dot leaders and all.
fn child_line(line: &MapLine) -> String {
    let title = &line.title;
    let width = title.chars().count();

    // At least one dot, so a title longer than the column still reads as a
    // leader rather than running straight into the horizon.
    let dots = LEADER_COLUMN.saturating_sub(width + 1).max(1);

    // Priority leads the tail, because it is what the boards sort by and the
    // one thing the model cannot infer from anything else on the line. The
    // spec's worked example leaves it off a child line; its prose requires it
    // ("Every line carries priority and status"), and the prose is the half
    // the model depends on.
    format!(
        "  {title} {} {}, {}, {}",
        ".".repeat(dots),
        priority_label(line.priority),
        horizon_label(line.horizon),
        line.detail
    )
}

/// `p9`, or `p-` for work nobody has ranked.
///
/// Never `p0`. An absent priority means nobody ranked this; zero would mean
/// ranked lowest, and the boards already treat those as different — a task
/// with `priority IS NULL` never reaches the Priority board at all. Collapsing
/// the two here would teach the model a distinction the rest of the app spends
/// effort maintaining.
fn priority_label(priority: Option<i64>) -> String {
    match priority {
        Some(p) => format!("p{p}"),
        None => "p-".to_string(),
    }
}

fn horizon_label(horizon: TaskHorizon) -> &'static str {
    match horizon {
        TaskHorizon::Daily => "daily",
        TaskHorizon::Weekly => "weekly",
        TaskHorizon::Monthly => "monthly",
        TaskHorizon::LongTerm => "long-term",
    }
}

/// Statuses in the words a person would use.
///
/// `planned` and `backlog` both read as "not started": the difference between
/// them is a queue position the model has no use for, and spending a token on
/// it would buy nothing.
fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Backlog | TaskStatus::Planned => "not started",
        TaskStatus::InProgress => "in progress",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Completed => "done",
        TaskStatus::Cancelled => "cancelled",
        TaskStatus::Deferred => "deferred",
    }
}

/// Fills the map up to `budget`, returning what fitted and how many sections
/// were dropped.
///
/// Both caps apply at once: a section is admitted only if it fits the token
/// budget *and* leaves the line count within [`MAX_LINES`] *and* the section
/// count within [`MAX_SECTIONS`]. The token budget alone would let a hundred
/// terse headings through; the count caps alone would let twenty verbose ones
/// blow the context.
///
/// The truncation notice is charged before anything is admitted, so saying
/// that the map is incomplete can never itself be what makes it overflow.
///
/// Dropping stops at the first section that does not fit rather than
/// continuing to look for a smaller one further down. The list is in rank
/// order, so skipping ahead would put a low-ranked section in a map that had
/// just refused a higher-ranked one — which is precisely the inversion the
/// ranking exists to prevent.
fn fit_within(sections: Vec<MapSection>, budget: usize) -> (Vec<MapSection>, usize) {
    let total = sections.len();
    let reserve = estimate_tokens(&truncation_notice(total));
    let mut spent = estimate_tokens(HEADER) + reserve;

    let mut lines = 0;
    let mut kept: Vec<MapSection> = Vec::new();

    for section in sections {
        if kept.len() >= MAX_SECTIONS || lines + section.line_count() > MAX_LINES {
            break;
        }

        let cost = estimate_tokens(&section.render());
        if spent + cost > budget {
            break;
        }

        spent += cost;
        lines += section.line_count();
        kept.push(section);
    }

    let dropped = total - kept.len();
    (kept, dropped)
}
