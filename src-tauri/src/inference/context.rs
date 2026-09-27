//! What the model is told, and how much of it fits.
//!
//! Three tiers, per spec §9.2, and the arithmetic is the whole point. A year
//! of accumulated tasks reaches a couple of hundred thousand tokens; at the
//! ~250 tok/s CPU prefill this app is sized for, sending it would cost several
//! minutes before the first word came back. So the model gets a small **map**
//! of what exists ([`super::map`], tier 1), a small set of **ranked detail**
//! (tier 2, below), and the ability to ask for one subtree at a time
//! (tier 3, [`fetch_task_subtree`], used by PR 28's expansion loop).
//!
//! **Budgets are enforced here, never hoped for in the template.** A prompt
//! that renders six thousand tokens into a two-thousand-token context is a
//! truncated prompt, and a truncated prompt is a model answering a question it
//! was never fully asked. Every tier is assembled by adding ranked items until
//! the next one would not fit, and then saying so.

use chrono::{NaiveDate, Weekday};
use rusqlite::Connection;
use serde::Serialize;

use super::map::{build_map, CommitmentMap};
use crate::domain::{format_minutes, month_containing, week_containing};
use crate::storage::{StorageError, Task, TaskHorizon, TaskRepo};

/// Tier 1's share of the budget, in estimated tokens (spec §9.2).
pub const DEFAULT_MAP_BUDGET: usize = 1_500;

/// Tier 2's share of the budget, in estimated tokens (spec §9.2).
pub const DEFAULT_CONTEXT_BUDGET: usize = 2_000;

/// Room kept for the model's own answer, in tokens.
///
/// The context window holds the prompt *and* the reply. A budget that filled
/// it with prompt would leave the model with nowhere to write, which
/// `llama-server` resolves by truncating — silently, and from the front, where
/// the system prompt lives.
pub const RESERVED_FOR_REPLY: usize = 512;

/// Room kept for the prompt template's own prose — the instructions wrapped
/// around the two tiers.
///
/// A number rather than a measurement because the budget has to be known
/// before a template is chosen. It is checked against the real templates by
/// `builtin_templates_fit_the_overhead_allowance`, so a template that outgrows
/// it fails a test instead of quietly stealing the map's budget.
pub const TEMPLATE_OVERHEAD: usize = 400;

/// How many context items may be sent, however small they are.
///
/// A second cap beside the token budget, for the same reason the map has one:
/// two hundred one-line items are within any budget and still unreadable to a
/// 1.7B model, which loses the thread long before it runs out of context.
pub const MAX_CONTEXT_ITEMS: usize = 24;

/// A task has to have moved this many times before it is worth raising.
///
/// Spec §10.3's reflection prompt is *"you have moved this five times — is it
/// blocked, too large, or no longer important?"*. Three is where that question
/// starts being fair: one move is a busy day, two is a bad week, three is a
/// pattern.
pub const REPEATED_DEFERRAL_THRESHOLD: i64 = 3;

/// How far ahead a due date counts as upcoming, in days.
pub const DUE_SOON_DAYS: i64 = 7;

/// Which session is being prepared.
///
/// Chooses the prompt template and the emphasis of the ranking, nothing more:
/// every session sees the same map, because "what am I committed to" does not
/// change with the time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionKind {
    DailyStandup,
    EveningReview,
    WeeklyPlanning,
    MonthlyReview,
}

impl SessionKind {
    /// The template file this session renders, without its extension.
    ///
    /// Only `daily-standup` ships today; the other three names are the files
    /// later PRs drop into `prompts/`. Naming them here rather than at the
    /// call site means adding one is a file plus a line in
    /// [`super::prompt::BUILTIN_TEMPLATES`], and no logic anywhere.
    pub fn template_name(self) -> &'static str {
        match self {
            Self::DailyStandup => "daily-standup",
            Self::EveningReview => "evening-review",
            Self::WeeklyPlanning => "weekly-planning",
            Self::MonthlyReview => "monthly-review",
        }
    }

    /// How this session is described to the model, in words.
    pub fn label(self) -> &'static str {
        match self {
            Self::DailyStandup => "daily standup",
            Self::EveningReview => "evening review",
            Self::WeeklyPlanning => "weekly planning",
            Self::MonthlyReview => "monthly review",
        }
    }
}

/// How many tokens each tier may spend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenBudget {
    pub map: usize,
    pub context: usize,
}

impl Default for TokenBudget {
    /// The spec's figures: 1,500 for the map, 2,000 for the ranked context.
    ///
    /// These suit the Balanced and High Quality profiles, whose context
    /// windows are 4k and 8k. A Lightweight machine cannot hold both — see
    /// [`TokenBudget::for_context_size`], which is what any caller holding a
    /// real profile should use.
    fn default() -> Self {
        Self {
            map: DEFAULT_MAP_BUDGET,
            context: DEFAULT_CONTEXT_BUDGET,
        }
    }
}

impl TokenBudget {
    /// The budget that fits inside a context window of `context_size` tokens.
    ///
    /// The spec's 1,500 + 2,000 is 3,500 tokens of content, which does not fit
    /// the Lightweight profile's 2k window at all — and this is exactly the
    /// place that failure has to be caught. Left to the template it would
    /// surface as a model that answers a question with its first third
    /// missing, which reads as a bad model rather than as a budget bug.
    ///
    /// Scaled three parts map to four parts ranked detail, matching the ratio
    /// the spec's own figures imply, so a small window loses proportionally
    /// from both tiers rather than losing one of them entirely. The map is
    /// what tells the model what exists; a session that dropped it would be a
    /// session about nothing.
    pub fn for_context_size(context_size: usize) -> Self {
        let usable = context_size
            .saturating_sub(RESERVED_FOR_REPLY)
            .saturating_sub(TEMPLATE_OVERHEAD);

        let map = (usable * 3 / 7).min(DEFAULT_MAP_BUDGET);
        let context = usable.saturating_sub(map).min(DEFAULT_CONTEXT_BUDGET);

        Self { map, context }
    }

    /// The two tiers added together.
    pub fn total(self) -> usize {
        self.map + self.context
    }
}

/// Where a context item came from.
///
/// **This enum is the Wave 7 seam.** A tier holds a list of context items, not
/// a list of tasks, so when the Obsidian indexer starts producing candidates
/// it adds a `Vault` variant here and a second gatherer beside
/// [`gather_items`] — and every consumer below, the ranker, the budget, the
/// renderer, the prompt, keeps working untouched. Nothing in this module may
/// reach through an item to a `Task`, which is why [`ContextItem`] carries
/// rendered `detail` strings rather than a task handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextSource {
    Task,
}

/// Why an item is in the context.
///
/// Doubles as the ranking key: the order of the variants is the order of
/// importance, and [`Self::weight`] turns it into a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextGroup {
    /// This month's commitment (spec §4).
    MonthlyCommitment,
    /// This week's milestone.
    WeeklyCommitment,
    /// Scheduled for yesterday or earlier and still not finished.
    Unfinished,
    /// Due within [`DUE_SOON_DAYS`].
    DueSoon,
    /// Moved at least [`REPEATED_DEFERRAL_THRESHOLD`] times (spec §11.2).
    RepeatedlyDeferred,
}

impl ContextGroup {
    /// The heading this group renders under.
    pub fn label(self) -> &'static str {
        match self {
            Self::MonthlyCommitment => "monthly commitment",
            Self::WeeklyCommitment => "weekly milestone",
            Self::Unfinished => "unfinished",
            Self::DueSoon => "due soon",
            Self::RepeatedlyDeferred => "repeatedly deferred",
        }
    }

    /// The group's share of an item's rank.
    ///
    /// Spaced a thousand apart so a group always outranks the one below it
    /// however high the task's own priority is. Commitments come first
    /// because a standup that dropped them to make room for a due date would
    /// be a standup about errands.
    fn weight(self) -> i64 {
        match self {
            Self::MonthlyCommitment => 5_000,
            Self::WeeklyCommitment => 4_000,
            Self::Unfinished => 3_000,
            Self::DueSoon => 2_000,
            Self::RepeatedlyDeferred => 1_000,
        }
    }
}

/// One thing the model is told about, with the detail that makes it mean
/// something.
///
/// The detail lines are the difference between "this is on your list" and
/// "this has moved four times and is blocked on a reply". Without them tier 2
/// would be a second, worse copy of the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextItem {
    /// The id the model quotes back to ask for more (tier 3). A vault-sourced
    /// item will carry its own id here; nothing downstream assumes a task.
    pub id: String,
    pub source: ContextSource,
    pub group: ContextGroup,
    pub title: String,
    /// Short phrases, rendered as a comma-separated tail.
    pub detail: Vec<String>,
    pub rank: i64,
}

impl ContextItem {
    /// The single line this item contributes to the prompt.
    pub fn render(&self) -> String {
        let mut line = format!("- [{}] {} ({})", self.id, self.title, self.group.label());

        if !self.detail.is_empty() {
            line.push_str(" — ");
            line.push_str(&self.detail.join("; "));
        }

        line
    }
}

/// Everything one session sends, both tiers, already inside budget.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionContext {
    pub kind: SessionKind,
    /// The day the session is about, ISO-8601.
    pub date: String,
    pub map: CommitmentMap,
    pub items: Vec<ContextItem>,
    /// Set when ranked items were dropped to fit the budget. The rendered text
    /// says so too — the model must not believe it saw everything.
    pub truncated: bool,
    pub dropped_items: usize,
    pub map_tokens: usize,
    pub context_tokens: usize,
}

impl SessionContext {
    /// Tier 2, as the prompt sees it.
    pub fn render_context(&self) -> String {
        if self.items.is_empty() {
            return "RANKED CONTEXT — nothing outstanding".to_string();
        }

        let mut out = String::from("RANKED CONTEXT");
        for item in &self.items {
            out.push('\n');
            out.push_str(&item.render());
        }

        if self.truncated {
            out.push('\n');
            out.push_str(&truncation_notice(self.dropped_items));
        }

        out
    }

    /// Both tiers added together.
    pub fn total_tokens(&self) -> usize {
        self.map_tokens + self.context_tokens
    }
}

/// Builds both tiers for one session, inside `budget`.
///
/// `today` and `week_starts_on` are arguments rather than read from the clock
/// and the settings row, because Rust owns dates (spec §3.6) and because a
/// builder that consulted `Local::now()` internally could not be tested
/// against a fixed board. The caller that has a real user has both values
/// already.
///
/// Deviates from the spec's `build_context(session_kind, budget)` by taking
/// the repository, the connection and the date: the spec sketch predates the
/// decision to feed every tier from SQLite rather than from a vault index.
pub fn build_context(
    repo: &TaskRepo,
    conn: &Connection,
    kind: SessionKind,
    budget: TokenBudget,
    today: NaiveDate,
    week_starts_on: Weekday,
) -> Result<SessionContext, StorageError> {
    let map = build_map(repo, conn, budget.map)?;
    let active = repo.list_active(conn)?;

    let mut candidates = gather_items(&active, today, week_starts_on);

    // Highest rank first, then by id, so an overflow drops the same items
    // every time. A ranking that varied between runs would make the "why did
    // it not mention X" question unanswerable.
    candidates.sort_by(|a, b| b.rank.cmp(&a.rank).then_with(|| a.id.cmp(&b.id)));

    let (items, dropped) = fit_within(candidates, budget.context);
    let truncated = dropped > 0;

    let map_tokens = estimate_tokens(&map.render());

    let mut context = SessionContext {
        kind,
        date: today.format("%Y-%m-%d").to_string(),
        map,
        items,
        truncated,
        dropped_items: dropped,
        map_tokens,
        context_tokens: 0,
    };

    context.context_tokens = estimate_tokens(&context.render_context());

    Ok(context)
}

/// Turns the active tasks into ranked candidates, one per task.
///
/// A task can qualify for several groups at once — a monthly commitment can
/// also be overdue and thrice-deferred — and it is listed once, under its
/// highest group, with the rest of the story in its detail lines. Listing it
/// twice would spend budget saying the same thing and would read to the model
/// as two separate commitments.
fn gather_items(active: &[Task], today: NaiveDate, week_starts_on: Weekday) -> Vec<ContextItem> {
    let month = month_containing(today);
    let week = week_containing(today, week_starts_on);
    let horizon = today
        .checked_add_signed(chrono::Duration::days(DUE_SOON_DAYS))
        .unwrap_or(today)
        .format("%Y-%m-%d")
        .to_string();
    let today_iso = today.format("%Y-%m-%d").to_string();

    active
        .iter()
        .filter_map(|task| {
            let group = classify(task, &month.start, &month.end, &week.start, &week.end)
                .or_else(|| overdue_group(task, &today_iso, &horizon))?;

            Some(ContextItem {
                id: task.id.clone(),
                source: ContextSource::Task,
                group,
                title: task.title.clone(),
                detail: detail_lines(task),
                rank: rank_of(group, task),
            })
        })
        .collect()
}

/// The commitment groups: this month's and this week's.
fn classify(
    task: &Task,
    month_start: &str,
    month_end: &str,
    week_start: &str,
    week_end: &str,
) -> Option<ContextGroup> {
    match task.horizon {
        TaskHorizon::Monthly if overlaps(task, month_start, month_end) => {
            Some(ContextGroup::MonthlyCommitment)
        }
        TaskHorizon::Weekly if overlaps(task, week_start, week_end) => {
            Some(ContextGroup::WeeklyCommitment)
        }
        _ => None,
    }
}

/// The attention groups: work that has slipped, is about to land, or keeps
/// moving.
///
/// Checked in that order, and the order is the point: something scheduled for
/// last Tuesday and still open is a more urgent thing to say out loud than
/// something due on Friday.
fn overdue_group(task: &Task, today: &str, horizon: &str) -> Option<ContextGroup> {
    // "Yesterday's incomplete tasks" per spec §9.2, read as "scheduled for any
    // day before today and still open". A task last scheduled three days ago
    // is more unfinished than yesterday's, not less, and a strict yesterday
    // filter would hide exactly the ones that have been ignored longest.
    if task
        .scheduled_date
        .as_deref()
        .is_some_and(|date| date < today)
    {
        return Some(ContextGroup::Unfinished);
    }

    if task
        .due_date
        .as_deref()
        .is_some_and(|date| date >= today && date <= horizon)
    {
        return Some(ContextGroup::DueSoon);
    }

    if task.rollover_count >= REPEATED_DEFERRAL_THRESHOLD {
        return Some(ContextGroup::RepeatedlyDeferred);
    }

    None
}

/// Whether a task's period overlaps `[start, end]`.
///
/// Overlap rather than containment, matching
/// [`crate::storage::TaskRepo::list_for_period`]: a commitment running across
/// a month boundary belongs to both months it touches.
fn overlaps(task: &Task, start: &str, end: &str) -> bool {
    match (task.period_start.as_deref(), task.period_end.as_deref()) {
        (Some(from), Some(to)) => from <= end && to >= start,
        // A commitment with no dates on it is this period's by default. The
        // alternative is to drop the one thing the user typed without ever
        // opening a date picker.
        _ => true,
    }
}

/// An item's place in the queue.
///
/// Group first and by a wide margin, then the user's own priority, then how
/// many times the task has moved. The rollover term is what pushes a
/// low-priority task that has been dodged four times above an untouched one
/// beside it — which is the whole reason §11.2 asks for the number.
fn rank_of(group: ContextGroup, task: &Task) -> i64 {
    group.weight() + task.priority.unwrap_or(0) * 10 + task.rollover_count.clamp(0, 50)
}

/// The phrases that make an item worth reading.
///
/// Ordered worst-news-first. A model reading a truncated line still sees the
/// blocker.
fn detail_lines(task: &Task) -> Vec<String> {
    let mut detail = Vec::new();

    if let Some(blocker) = task.blocker.as_deref().filter(|b| !b.trim().is_empty()) {
        detail.push(format!("blocked: {}", blocker.trim()));
    }

    if let Some(target) = task.progress_target {
        let current = task.progress_current.unwrap_or(0.0);
        let unit = task.progress_unit.as_deref().unwrap_or("").trim();
        let progress = format!("{}/{}", number(current), number(target));
        detail.push(if unit.is_empty() {
            progress
        } else {
            format!("{progress} {unit}")
        });
    }

    if let Some(priority) = task.priority {
        detail.push(format!("p{priority}"));
    }

    if let Some(due) = task.due_date.as_deref() {
        detail.push(format!("due {due}"));
    }

    if task.rollover_count > 0 {
        detail.push(format!("moved {}×", task.rollover_count));
    }

    if let Some(minutes) = task.time_spent_minutes.filter(|m| *m > 0) {
        detail.push(format!("{} logged", format_minutes(minutes)));
    }

    detail
}

/// Fills the tier up to `budget`, returning what fitted and how much did not.
///
/// The truncation notice is charged against the budget before anything else,
/// so admitting that items were dropped can never itself be the thing that
/// overflows.
fn fit_within(candidates: Vec<ContextItem>, budget: usize) -> (Vec<ContextItem>, usize) {
    let total = candidates.len();
    let reserve = estimate_tokens(&truncation_notice(total));
    let mut spent = estimate_tokens("RANKED CONTEXT") + reserve;

    let mut kept = Vec::new();
    for item in candidates {
        if kept.len() >= MAX_CONTEXT_ITEMS {
            break;
        }

        let cost = estimate_tokens(&item.render());
        if spent + cost > budget {
            break;
        }

        spent += cost;
        kept.push(item);
    }

    let dropped = total - kept.len();
    (kept, dropped)
}

/// What the model is told when the budget ran out.
///
/// Present tense and explicit, because the alternative is a list that simply
/// stops: the model then reasons as though it has seen everything, and
/// confidently tells the user that nothing else is outstanding.
pub fn truncation_notice(dropped: usize) -> String {
    format!(
        "... {dropped} more not shown — the context budget was reached, so this list is incomplete"
    )
}

/// Tier 3: one task with the context that makes it answerable.
///
/// A subtree rather than a row, because the interesting question is almost
/// never about one task in isolation — and because a weekly milestone means
/// little without the monthly commitment it sits under.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSubtree {
    pub task: Task,
    /// Direct children only. The hierarchy is monthly → weekly → daily
    /// (spec §4), so one level down from any task is the level that explains
    /// it; a full recursion would re-import the whole tree the tiers exist to
    /// avoid sending.
    pub children: Vec<Task>,
    /// Nearest parent first, up to [`MAX_ANCESTOR_DEPTH`].
    pub ancestors: Vec<Task>,
    pub notes: Option<String>,
    pub blocker: Option<String>,
}

/// How far up a parent chain the walk will go before it stops.
///
/// The real hierarchy is three deep. This cap is not about the real one — it
/// is about a `parent_task_id` cycle, which the schema permits and which would
/// otherwise spin this walk until the app was killed. A cycle also trips the
/// visited set below; the depth cap is the second lock on the same door,
/// because a chain a thousand long is just as unusable as a loop and does not
/// need to be a bug to be worth refusing.
pub const MAX_ANCESTOR_DEPTH: usize = 16;

/// Fetches one task, its children, its ancestors, its notes and its blocker.
///
/// Used by PR 28's expansion loop, where the model quotes back an id it saw in
/// the map. A missing id is an error rather than an empty subtree: the caller
/// is validating model output, and "no such task" is exactly the answer it
/// needs to refuse the expansion.
pub fn fetch_task_subtree(
    repo: &TaskRepo,
    conn: &Connection,
    id: &str,
) -> Result<TaskSubtree, StorageError> {
    let task = repo
        .get(conn, id)?
        .ok_or_else(|| StorageError::TaskNotFound { id: id.to_string() })?;

    let children = repo.children_of(conn, &task.id)?;

    let mut ancestors = Vec::new();
    let mut seen = vec![task.id.clone()];
    let mut cursor = task.parent_task_id.clone();

    while let Some(parent_id) = cursor {
        if ancestors.len() >= MAX_ANCESTOR_DEPTH || seen.contains(&parent_id) {
            break;
        }

        let Some(parent) = repo.get(conn, &parent_id)? else {
            break;
        };

        seen.push(parent.id.clone());
        cursor = parent.parent_task_id.clone();
        ancestors.push(parent);
    }

    Ok(TaskSubtree {
        notes: task.notes.clone(),
        blocker: task.blocker.clone(),
        task,
        children,
        ancestors,
    })
}

/// Formats a progress figure without a trailing `.0` on whole numbers.
///
/// "12/20 applications" is what the user wrote; "12.0/20.0 applications" is
/// what a float prints, and it costs tokens to say something less clearly.
pub fn number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

/// Estimates how many tokens a string will cost.
///
/// **This is an approximation, and a deliberately pessimistic one.** There is
/// no tokenizer in this process — the vocabulary lives inside the GGUF file,
/// which is only loaded when a session is running, and the budget has to be
/// computed before then. So this counts characters by class and applies the
/// worst plausible rate for each.
///
/// The two failure modes are not symmetric, which is what decides every
/// constant below:
///
/// - **Over-estimating** drops a line that would have fitted. The model sees
///   slightly less than it could have, and is told the list was truncated.
/// - **Under-estimating** overflows the context window. `llama-server` then
///   truncates the prompt itself, from the front, without telling anyone — so
///   the model answers a question whose beginning it never saw, and the
///   symptom is a bad answer rather than an error.
///
/// The rates:
///
/// - **ASCII letters, 0.5 each.** Byte-pair vocabularies merge letter runs
///   aggressively; two characters per token is the floor for real words and
///   holds for unbroken nonsense strings too.
/// - **ASCII digits, 1.0 each.** Llama-family tokenizers split numbers into
///   individual digits. `2026` is four tokens, not one.
/// - **ASCII punctuation and symbols, 1.0 each.** These merge least reliably;
///   a run of dot leaders or arrows is the worst case in the map's own output.
/// - **ASCII whitespace, 0.5 each.** A space usually merges into the word that
///   follows it, so it is rarely a token of its own.
/// - **Anything non-ASCII, one token per UTF-8 byte.** The true worst case: a
///   vocabulary with no merge for a character falls back to its bytes. CJK is
///   three bytes per character and emoji four, so a CJK heading is counted at
///   three tokens per character where a Qwen tokenizer would charge about one.
///   That is a large over-estimate and it is the right one — a budget is not
///   the place to bet on a vocabulary this process cannot read.
///
/// On ordinary English prose this over-counts by roughly two and a half times.
/// That is the price of the guarantee, and it is paid in map lines rather than
/// in correctness.
///
/// **What replaces this:** `llama-server` exposes a `/tokenize` endpoint, and
/// every completion already returns a real `prompt_tokens` count (see
/// [`crate::inference::client::Completion`]). Once a session is up, the true
/// count is one loopback call away, and the intended follow-up is to keep this
/// function for the pre-flight budget and reconcile against the real number
/// once the server is running.
pub fn estimate_tokens(text: &str) -> usize {
    let mut halves: usize = 0;

    for ch in text.chars() {
        halves += if !ch.is_ascii() {
            // Byte fallback: the worst a tokenizer can do with a character it
            // has no merge for.
            ch.len_utf8() * 2
        } else if ch.is_ascii_alphabetic() || ch.is_ascii_whitespace() {
            // Letters merge into words; a space merges into the word after
            // it. Two different reasons, and they land on the same rate —
            // half a token each.
            1
        } else {
            // Digits, punctuation, symbols, control characters: the classes
            // that merge least reliably, charged in full.
            2
        };
    }

    halves.div_ceil(2)
}

impl TaskSubtree {
    /// The subtree as the model reads it back.
    ///
    /// Plain lines rather than JSON: this is appended to a prompt the model has
    /// already been reading in prose, and switching representation mid-prompt
    /// costs tokens and invites the model to answer in JSON too.
    pub fn render(&self) -> String {
        let mut out = format!("TASK DETAIL — {}\n", self.task.title);

        out.push_str(&format!(
            "  {}, {}\n",
            self.task.horizon.as_str(),
            self.task.status.as_str()
        ));

        if let Some(blocker) = &self.blocker {
            // First after the headline: a blocker is the single most likely
            // reason the model asked about this task at all.
            out.push_str(&format!("  blocked: {blocker}\n"));
        }

        for ancestor in &self.ancestors {
            out.push_str(&format!("  part of: {}\n", ancestor.title));
        }

        for child in &self.children {
            out.push_str(&format!(
                "  contains: {} ({})\n",
                child.title,
                child.status.as_str()
            ));
        }

        if let Some(notes) = &self.notes {
            out.push_str(&format!("  notes: {notes}\n"));
        }

        out
    }
}

/// The marker the prompt tells the model to use when it wants one task in full.
pub const NEEDS_CONTEXT: &str = "NEEDS_CONTEXT";

/// The task id a reply is asking for, if it is asking for one.
///
/// Deliberately strict about the *shape* and forgiving about the punctuation:
/// the reply must begin with the marker, because a reply that merely mentions
/// needing context is prose and answering it with a fetch would swallow a real
/// answer. Within that, brackets are optional — the same model emits
/// `NEEDS_CONTEXT [id]` and `NEEDS_CONTEXT id` on the same prompt, and a parser
/// that accepted one shape would make the feature work intermittently, which
/// is worse than not working at all.
///
/// Returns `None` when nothing usable follows the marker rather than guessing,
/// so an unparseable request degrades to showing the model's own words instead
/// of fetching an arbitrary task.
pub fn needs_context_id(reply: &str) -> Option<String> {
    let rest = reply.trim().strip_prefix(NEEDS_CONTEXT)?;

    let id: String = rest
        .trim()
        .trim_matches(|c: char| c == '[' || c == ']' || c == '"' || c == '\'')
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string();

    if id.is_empty() {
        return None;
    }

    Some(id)
}

#[cfg(test)]
mod estimator_guarantees {
    //! The estimator's contract, stated as inequalities.
    //!
    //! There is no tokenizer to compare against, so these assert against the
    //! documented *lower bounds* of what a real tokenizer could charge —
    //! which is the only thing that matters here. Being far above the true
    //! count is the design; being below it even once is the bug.

    use super::estimate_tokens;

    #[test]
    fn an_empty_string_costs_nothing() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn cjk_text_is_charged_at_least_one_token_per_character() {
        // Worst realistic case for CJK is one token per character; absolute
        // worst is byte fallback at three. The estimate must clear the first
        // comfortably and meet the second.
        let text = "今日の予定を確認してください";
        let characters = text.chars().count();

        let estimate = estimate_tokens(text);

        assert!(
            estimate >= characters * 3,
            "{characters} CJK characters must cost at least {} tokens, got {estimate}",
            characters * 3
        );
    }

    #[test]
    fn emoji_are_charged_at_least_one_token_per_utf8_byte() {
        let text = "🚀🚀🚀";

        assert!(estimate_tokens(text) >= text.len());
    }

    #[test]
    fn a_long_unbroken_string_is_never_under_counted() {
        // A hundred characters no vocabulary has a merge for would cost at
        // most a hundred tokens; two characters per token is the floor for a
        // byte-pair vocabulary on ASCII letters.
        let text = "a".repeat(100);

        assert!(estimate_tokens(&text) >= 50);
    }

    #[test]
    fn heavy_punctuation_is_charged_one_token_per_character() {
        // Punctuation is where merges are least reliable, and the map's own
        // dot leaders are exactly this shape.
        let text = "....!!!???---===~~~^^^";

        assert!(
            estimate_tokens(text) >= text.chars().count(),
            "punctuation must not be assumed to merge"
        );
    }

    #[test]
    fn digits_are_charged_one_token_each_because_tokenizers_split_them() {
        let text = "1234567890";

        assert!(estimate_tokens(text) >= 10);
    }

    #[test]
    fn a_mixed_worst_case_line_is_not_under_counted() {
        // Every class at once, of the shape the map actually emits.
        let text = "  应用 2026 ....... weekly, 3/5 done ✅";
        let floor: usize = text
            .chars()
            .map(|c| if c.is_ascii_alphabetic() { 0 } else { 1 })
            .sum();

        assert!(
            estimate_tokens(text) >= floor,
            "every non-letter must be worth at least a token"
        );
    }

    #[test]
    fn ordinary_prose_is_over_counted_rather_than_under_counted() {
        // Roughly four characters per token is the usual English rate. The
        // estimate must sit above it, not below.
        let text = "Rewrite the resume and send three referral follow-ups this week.";

        assert!(estimate_tokens(text) > text.len() / 4);
    }

    #[test]
    fn the_estimate_grows_with_the_text() {
        let short = estimate_tokens("Job Search");
        let long = estimate_tokens("Job Search and everything filed underneath it");

        assert!(long > short);
    }
}
