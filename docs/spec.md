---
title: Local AI Daily Standup and Sticky-Note Planner
type: Product and Technical Specification
status: Concept
version: 0.1
last_updated: 2026-08-25
license: MIT
---

# Local AI Daily Standup and Sticky-Note Planner

## Executive Summary

This project is an open-source, local-first desktop application that helps a person plan and review their life through private voice conversations.

The application holds the user's goals, commitments, and tasks itself, and uses them to conduct personal daily standups, weekly planning sessions, monthly planning sessions, and retrospective reviews.

It can also connect to an Obsidian vault containing long-term goals, projects, priorities, and tasks, and draw on that material during the same sessions. The vault is an enrichment rather than a prerequisite: everything above works before one is connected, and Obsidian integration is the last part of the roadmap (§22).

After a planning conversation, the application converts approved commitments into lightweight desktop sticky-note boards.

The local LLM, speech recognition engine, and text-to-speech engine only run while an AI-assisted conversation or review is taking place. Once the session finishes, those resource-intensive processes terminate completely. The desktop sticky notes remain available through a separate lightweight process.

The intended hierarchy is:

```text
Long-term Obsidian goals
        ↓
Monthly commitments
        ↓
Weekly milestones
        ↓
Daily actions
        ↓
Completed work and reflections
        ↓
Weekly/monthly progress written back to Obsidian
```

This is the finished shape, not the first release. The top and bottom rows are the Obsidian phases; until those ship, the application holds the long-term goals itself and keeps the progress it produces locally. Every row between them works from day one.

---

# 1. Product Vision

Create a private personal planning companion that turns long-term goals into actions that can realistically be completed today.

The application should feel like having a short standup with an organized version of yourself. It should:

- Remember what matters — in its own goals and commitments, and through an Obsidian vault once one is connected.
- Ask useful questions using voice.
- Help identify realistic commitments.
- Keep those commitments visible on the desktop.
- Track daily, weekly, and monthly progress.
- Notice recurring blockers and repeatedly deferred tasks.
- Support reflection without requiring cloud services.
- Keep the user in control of every change made to their notes.

The project should be useful even when the AI features are not running.

---

# 2. Problem

Long-term goals are often stored in documents, journals, or Obsidian notes, while daily work happens somewhere else.

This creates several problems:

- Long-term goals are easy to forget during daily planning.
- Task applications often contain actions without meaningful context.
- Planning systems require too much manual maintenance.
- Daily tasks do not clearly contribute to weekly or monthly outcomes.
- Unfinished tasks are repeatedly carried forward without reflection.
- AI planning tools commonly require private notes to be sent to cloud services.
- Local AI applications may waste RAM or GPU memory by keeping models loaded all day.
- Traditional sticky notes provide visibility but little structure or reflection.

This project connects long-term direction, short-term planning, desktop visibility, and private AI assistance.

---

# 3. Core Product Principles

## 3.1 Local First

All core functionality must work without an internet connection after the required models have been downloaded.

Private content should remain on the user's computer:

- Obsidian notes
- Voice recordings
- Transcripts
- Daily plans
- Weekly reviews
- Monthly reviews
- Model prompts and responses

Cloud integrations may be considered later, but they must remain optional.

## 3.2 Obsidian Is the Long-Term Source of Truth

Obsidian stores:

- Long-term goals
- Projects
- Areas of life
- Priorities
- Project statuses
- Detailed plans
- Permanent reflections
- Long-term task backlogs

The desktop application stores operational planning information such as:

- Today's tasks
- This week's commitments
- This month's commitments
- Sticky-note positions
- Window sizes
- Display preferences
- Temporary conversation state

This principle describes where long-term material *belongs* once a vault is connected, not when the vault arrives. Obsidian integration is the last thing this product builds (§22), and until it is connected the application holds its own goals, commitments and named task groups directly. A user who never connects a vault still has somewhere to put long-term direction; a user who does connect one gets the durable, linkable, greppable home that a Markdown vault is, and the application's copy defers to it. The rule stands either way: the app owns what is operational and current, and the vault owns what is permanent.

## 3.3 AI Is On Demand

The LLM must not remain loaded simply because sticky notes are visible.

The application should separate:

1. A lightweight sticky-note host.
2. The main planning interface.
3. Local LLM inference.
4. Speech-to-text.
5. Text-to-speech.

The AI processes start only when requested and terminate after the session.

## 3.4 Human Approval Before Writing

The application must never silently rewrite an Obsidian note.

Before writing, it should show:

- The affected file.
- The proposed addition or modification.
- Why the change is being proposed.
- An approve, edit, or reject option.

## 3.5 Current Commitments Must Remain Focused

Not every Obsidian task should become a desktop sticky note.

Obsidian represents the complete backlog. Sticky notes represent current commitments.

The application should encourage:

- A small number of daily actions.
- A manageable weekly plan.
- A limited number of monthly outcomes.
- Clear connections between planning levels.

## 3.6 Deterministic Application, Assisted by AI

The LLM should not control the entire application.

Regular application code should control:

- Session stages
- File access
- Task status changes
- Date calculations
- Progress calculations
- Database writes
- Model startup and shutdown
- Obsidian write approval

The LLM should help with:

- Natural conversation
- Follow-up questions
- Summarization
- Task extraction
- Breaking goals into actions
- Detecting possible conflicts or blockers
- Drafting reflections

---

# 4. Planning Hierarchy

## 4.1 Long-Term Goals

Long-term goals live in the application until an Obsidian vault is connected, and in the vault afterwards (§3.2). The level exists from the first release either way, because monthly commitments need something above them to point at.

Examples:

- Find a new job.
- Improve physical health.
- Complete a personal software project.
- Prepare for a major trip.
- Improve financial organization.

These goals should not automatically become sticky notes. They provide direction for monthly commitments.

## 4.2 Monthly Commitments

Monthly commitments describe meaningful outcomes to reach during the current month.

Examples:

- Submit 20 strong job applications.
- Finish the MVP of the daily standup application.
- Schedule all overdue health appointments.
- Book transportation and accommodations for a trip.

Monthly commitments should have measurable progress where possible.

## 4.3 Weekly Milestones

Weekly milestones move a monthly commitment forward.

Examples:

- Submit five applications this week.
- Complete the sticky-note prototype.
- Schedule the dental and vision appointments.
- Compare three flight and hotel options.

## 4.4 Daily Actions

Daily actions are small, concrete tasks that can reasonably be completed today.

Examples:

- Customize the résumé for Company X.
- Implement sticky-note window persistence.
- Call the dental clinic.
- Compare two hotels.

## 4.5 Linked Planning Example

```text
Long-term goal:
Find a new job

Monthly commitment:
Submit 20 strong applications

Weekly milestone:
Submit five applications this week

Daily action:
Customize résumé and apply to Company X
```

Completing a daily task contributes to the weekly milestone. Weekly progress contributes to the monthly commitment. Monthly reviews summarize progress toward the long-term goal, wherever that goal is currently stored.

---

# 5. Primary User Experience

## 5.1 First-Time Setup

During onboarding, the application should:

1. Explain that all AI processing is local.
2. Ask where sticky-note boards should appear.
3. Offer to start with daily planning only or enable all planning horizons.
4. Invite the user to name a first few sections — the areas or projects their work falls into (§6.2).
5. Detect available CPU, RAM, and GPU capabilities.
6. Recommend suitable local models.
7. Download models only after confirmation.
8. Test the microphone and selected voice.

Onboarding must not ask for an Obsidian vault. Setup should end with usable boards, and it can only do that if every step in it is something the application itself can act on. A vault question at this point either blocks a user who does not keep one or collects a path that nothing will read for several releases; both make the first run feel like configuration rather than planning.

Connecting a vault is a later, optional step in Settings, added when the Obsidian phases ship (§22). That flow should then:

1. Ask the user to select a vault.
2. Request read-only access initially.
3. Scan the vault's Markdown structure.
4. Explain which files and task patterns it discovered.
5. Let the user exclude private folders.

## 5.2 Morning Standup

At a configurable time, the application displays a notification:

> Ready for your morning standup?

When the user starts the session:

1. The planning window opens.
2. Whisper and the selected LLM are launched.
3. Relevant context is retrieved: the user's goals, monthly commitments, weekly milestones, open tasks and sections, plus Obsidian excerpts once a vault is connected.
4. The assistant summarizes the current situation.
5. The assistant asks what happened yesterday.
6. It asks what matters today.
7. It asks about blockers.
8. It compares proposed work with weekly and monthly commitments.
9. It recommends a small set of daily actions.
10. The user approves or edits the plan.
11. Daily sticky notes are updated.
12. The standup summary is saved.
13. The LLM, Whisper, and voice processes terminate.

Example:

> Your job-search project has priority 9, and this week's target is five applications. You have completed two. You also carried the dental appointment forward three times. What would make today successful?

## 5.3 Evening Check-In

The evening check-in should be shorter than the morning standup.

The assistant asks:

- What was completed?
- What remains unfinished?
- What caused unfinished work?
- Should unfinished work be rescheduled, returned to the backlog, delegated, or removed?
- Is there anything worth recording for tomorrow?

The app should distinguish between:

- A task that remains important.
- A task blocked by another person or event.
- A task that was too large.
- A task the user no longer wants to complete.
- A task that has become a recurring avoidance pattern.

## 5.4 Weekly Planning

A weekly planning session should:

1. Review the previous week.
2. Calculate planned versus completed work.
3. Identify tasks repeatedly carried forward.
4. Review monthly commitments.
5. Select realistic milestones for the new week.
6. Break large milestones into possible daily actions.
7. Let the user approve the weekly plan.
8. Save the weekly review, and write it to Obsidian once writeback is available (§22).
9. Refresh the weekly sticky-note boards.

Suggested questions:

- What went well last week?
- What took more time than expected?
- Which blockers are still active?
- Which monthly commitment most needs progress?
- What are the three most important outcomes this week?
- What should deliberately not be worked on this week?

## 5.5 Monthly Planning

A monthly planning session should:

1. Review the previous month's commitments.
2. Compare completed work with long-term goals.
3. Identify neglected life or project areas.
4. Review whether priorities have changed.
5. Select a limited number of monthly commitments.
6. Define measurable outcomes.
7. Create initial weekly milestones.
8. Save the monthly plan and retrospective, and write them to Obsidian once writeback is available (§22).

Suggested questions:

- Which long-term goals made meaningful progress?
- Which goals received no attention?
- What repeatedly blocked progress?
- Which goal is no longer important?
- What should success look like by the end of this month?
- Which commitments are realistic given available time?

---

# 6. Sticky-Note Desktop Experience

## 6.1 Visual Direction

The interface should resemble a set of dark desktop boards with subtle accent colors.

The initial layout should support boards similar to:

- Priority Tasks
- Weekly Tasks
- Weekly Progress
- Monthly Tasks
- Monthly Progress

Each board should appear as a separate frameless desktop window.

Suggested visual characteristics:

- Dark charcoal background.
- Light text.
- Thin colored top border.
- Soft rounded corners.
- Minimal controls until the pointer hovers over the board.
- Clear section dividers.
- Compact typography.
- Optional translucent background.
- User-selectable accent colors.
- Strong contrast and readable font sizes.

## 6.2 Sections

A **section** is a user-named group of tasks on a board. On the Priority Tasks board a section is a task's `area`; on the Weekly Tasks board it is its `project` (§10). Those headings already existed — "Job search", "Health", "Travel" in the examples below — but they were derived: the board read the `area` values off the tasks and drew a heading for each distinct one. A section is the same heading, declared deliberately by the user instead of inferred from whatever happens to be filed there.

### Why Declaring Matters

A derived group exists only for as long as some task carries its name. That is fine for a group that is already full and useless for one that is not. A user who wants a place to put next month's job-search work has to invent the heading and fill it in the same motion, because a heading typed and not yet filled has nothing holding it up — it would vanish between being named and being used.

Declaring separates the two acts:

- A declared section stays on the board while it is empty.
- It can be named now and filled later, or over several days.
- It survives its last task being completed, moved, or deleted.
- It is a thing the user can rename and delete, rather than a side effect of the tasks underneath it.

Empty sections sort first. This looks backwards — the empty group is the one with nothing to show — but it follows from what the user just did. The only reason to create an empty section is to put something in it, and a section that appears below a screenful of existing work reads as the button having done nothing at all. The new heading should be where the user is already looking.

### What Sits Inside One

Ordinary tasks, and nothing else. A task under a section is the same task described in §10, with its priority, recorded time, completion state, and the full §6.7 interaction set. There is no second kind of item — no section-only note, no heading-level checkbox, no summary row that behaves differently from its neighbours. One task type keeps every board rule, every rollover count, and every progress calculation working the same way regardless of where a task is filed.

Example:

```text
PRIORITY TASKS

Interviews                        (empty)

Job search
P9  ☐  Submit remaining applications        —
P6  ☑  Rewrite the cover letter template   45m

Health
P8  ☐  Schedule dental appointment           —
```

### Renaming

Renaming a section rewrites the `area` or `project` of every task inside it, in a single transaction.

This has to be atomic. A partial rename leaves some tasks answering to the old name and some to the new one, and because the headings are still derived from those values, the board draws both — the group visibly splits in two, with the same work under two names. There is no state in which a half-renamed section is a reasonable thing to show, so the rename either applies to every member or to none.

### Deleting

Deleting a section **unfiles** its tasks. It does not delete them.

The tasks lose their `area` or `project` and fall into Unsorted, where they remain visible and can be filed somewhere else. Tidying a heading must never destroy work: the user's intent in removing a section is that the grouping was wrong, which says nothing about whether the tasks under it still matter. Making deletion destructive would mean the price of reorganising a board is remembering what was in it.

### Name Matching

Section names match case-insensitively and with surrounding whitespace trimmed. "Job Search", "job search" and `job search ` are one section, not three.

The alternative is a board that quietly accumulates near-duplicate headings from ordinary typing, and a user who cannot tell why their tasks landed in two places. The name the user typed is preserved for display; only the comparison is normalised.

### Which Boards Take Sections

- **Priority Tasks** — yes, grouped by `area`.
- **Weekly Tasks** — yes, grouped by `project`.
- **Weekly Progress** — no. It groups by day.
- **Monthly Progress** — no. It groups by commitment.

The progress boards are excluded because their groupings are not names a user can invent. Monday is Monday whether or not anything is scheduled on it, and a monthly commitment is a task in its own right with a target and a measured result. Allowing a declared section alongside either one would put two competing grouping schemes on the same board.

### Priority and Ordering

A task's priority is settable from 1 to 10 directly from its priority badge, without opening an editor, and the boards re-sort as soon as it changes.

Sections order by their highest-priority member. Raising one task can therefore lift its whole section up the board, which is usually the point: when one thing inside a group becomes urgent, the group is where the user's attention needs to go. Empty sections keep their place at the top regardless, since they have no member to take a priority from.

## 6.3 Priority Tasks Board

The Priority Tasks board contains important items that may span more than one day.

Example:

```text
PRIORITY TASKS

Job search
• Submit remaining applications

Health
• Schedule dental appointment
• Book comprehensive eye exam

Personal
• Transfer email accounts
```

Priority tasks are created or promoted manually, and may additionally originate from Obsidian once a vault is connected.

Its headings are `area` values, and each one may be a declared section (§6.2).

## 6.4 Weekly Tasks Board

The Weekly Tasks board shows outcomes expected during the current week.

Example:

```text
WEEKLY TASKS

Job Search
☐ Submit five applications
☐ Email two professional contacts

Daily Standup Project
☐ Build sticky-note prototype
☐ Test model shutdown behavior

Travel
☐ Choose hotel
```

Its headings are `project` values, and each one may be a declared section (§6.2).

## 6.5 Weekly Progress Board

The Weekly Progress board shows the whole week, Monday through Sunday, as a set of collapsible days.

Every day is always listed, even when empty, so the week reads as a complete shape rather than a list of only the busy days. Each day header summarises what is inside it without needing to be opened: how many tasks are done, and how much time was spent.

### The Week Label

The board labels its week as a date range followed by the week number:

```text
Aug 24-30 - Week 35
```

This line has one job: answer "which week am I looking at?" at a glance. An ISO identifier answers it precisely but not quickly. Rendering the same week as `2026-W35 2026-08-24 -> 2026-08-30` puts three near-identical strings of digits on the line, and the reader has to parse all three to extract the one fact they wanted. The month name breaks the run of numbers, and the range is what people actually recognise a week by.

The week number is kept because it is how the week is named everywhere else in the product — in frontmatter, in filenames, and in note titles (§13) — and dropping it from the board would leave nothing connecting the board to the note it produces. It is placed last, as the technical identifier, rather than first, as the headline.

Only the on-screen board label changed. ISO week identifiers remain the correct form wherever a week is being *identified* rather than *read*:

- Frontmatter (`week: 2026-W35`).
- Note filenames.
- Note titles.
- Any stored or exported reference to a week.

Collapsed:

```text
WEEKLY PROGRESS         Aug 17-23 - Week 34

▸ Monday      2/3    1h 45m
▾ Tuesday     1/2      35m
▸ Wednesday   0/2         —
▸ Thursday    0/0         —
▸ Friday      0/1         —
▸ Saturday    0/0         —
▸ Sunday      0/0         —
```

Expanded, a day reveals its tasks with priority, completion, and time:

```text
▾ Tuesday     1/2      35m

  P8  ☑  Complete onboarding task           35m
  P5  ☐  Schedule dental appointment           —
```

Each row shows four things:

- **Priority** — the task's importance, so the most significant work is identifiable at a glance rather than by reading every title.
- **A checkbox** — done or not.
- **Time spent** — how long it actually took, or a dash when nothing was recorded. Click to set or change it.

Today's day should be expanded by default and visually distinguished. The user's expand and collapse choices persist.

Completed items remain visible but dimmed or crossed out. This makes the board a record of the week rather than a list that empties as work is finished.

## 6.6 Monthly Board

A monthly board should focus on outcomes and progress rather than every individual task.

Example:

```text
AUGUST COMMITMENTS

JOB SEARCH
12 / 20 applications
████████████░░░░░░░░ 60%

DAILY STANDUP APP
MVP: 4 / 7 milestones complete
███████████░░░░░░░░░ 57%

HEALTH
1 / 3 appointments scheduled
██████░░░░░░░░░░░░░░ 33%
```

## 6.7 Sticky-Note Interactions

Users should be able to:

- Complete a task.
- Uncomplete a task.
- Edit task text.
- Set or change a task's priority from 1 to 10, using the task's priority badge.
- Record or change how long a task took.
- Expand and collapse a day or section.
- Create a named section (§6.2).
- Rename a section, rewriting every member task's area or project.
- Delete a section, unfiling its tasks into Unsorted.
- Move a task into a different section.
- Move a task to another day.
- Promote a daily task to weekly.
- Move a weekly task to another week.
- Link a task to an Obsidian note, once a vault is connected.
- Open the source note in Obsidian, once a vault is connected.
- Add a blocker.
- Add a short comment.
- Delete or archive a task.
- Ask the AI for help when explicitly requested.

Normal task operations, including recording how long something took, must not start the LLM.

## 6.8 Window Behaviors

Each board should support:

- Dragging.
- Resizing.
- Always-on-top mode.
- Desktop-level mode where supported.
- Locking the position.
- Click-through mode when locked.
- Collapsing to a title bar.
- Adjustable opacity.
- Adjustable font size.
- Pinning to a specific monitor.
- Remembering position and dimensions.
- Light and dark themes.
- Compact and expanded modes.
- Independent visibility settings.
- Restoring its previous state after reboot.

## 6.9 Tray Menu

The lightweight system-tray menu should provide:

```text
Open Boards
Start Daily Standup
Start Evening Check-In
Plan My Week
Monthly Review
Quick Add Task
Pause Reminders
Settings
Quit
```

Opening boards or adding a normal task should not load the LLM.

---

# 7. Application and Model Lifecycle

## 7.1 Required Separation

The application must separate the lightweight desktop experience from resource-intensive AI processes.

```text
Always available when enabled:
├── Sticky-note host
├── Tray icon
├── Local task database
└── File watcher

Only during an AI session:
├── Main assistant window
├── llama.cpp
├── whisper.cpp
└── Text-to-speech engine
```

The file watcher exists only to notice Obsidian notes changing underneath the index, so it runs only once a vault has been connected. Before then it is absent rather than idle, and the lightweight tier is the sticky-note host, the tray icon, and the task database.

## 7.2 Normal Desktop State

When only sticky notes are visible:

```text
Sticky-note host: Running
Tray icon: Running
Task database: Available
File watcher: Optional
LLM: Stopped
Whisper: Stopped
Text-to-speech: Stopped
GPU memory used by models: None
```

## 7.3 Standup State

When a standup starts:

1. Open the assistant interface.
2. Launch the selected LLM process.
3. Load the configured model.
4. Launch or initialize speech recognition.
5. Initialize text-to-speech.
6. Conduct the conversation.
7. Save approved results.
8. Refresh the boards.
9. Close the assistant interface.
10. Terminate all inference processes.
11. Confirm that model RAM and VRAM have been released.

## 7.4 Idle Shutdown

The application should automatically terminate AI processes after a configurable idle period.

Suggested default:

```text
Idle timeout: 5 minutes
```

Users may choose:

- Shut down immediately after each session.
- Shut down after 5 minutes.
- Keep loaded for 15 minutes.
- Keep loaded until manually stopped.

The default should favor resource conservation.

## 7.5 Startup Tradeoff

Because the model is unloaded between sessions, starting a standup may require several seconds.

The UI should communicate this clearly:

```text
Starting local assistant…
Loading language model…
Preparing microphone…
Ready.
```

This startup delay is preferable to consuming several gigabytes of RAM throughout the day.

---

# 8. System Architecture

## 8.1 Recommended Technology Stack

### Desktop Framework

**Tauri 2**

Responsibilities:

- Main application window.
- Sticky-note windows.
- System tray.
- Notifications.
- File-system permissions.
- Model process management.
- Application packaging.
- Cross-platform support.

Tauri supports bundling and launching external binaries as sidecars:

- <https://v2.tauri.app/develop/sidecar/>

It also supports system-tray applications:

- <https://v2.tauri.app/learn/system-tray/>

### Frontend

**React and TypeScript**

Responsibilities:

- Planning interface.
- Conversation display.
- Sticky-note boards.
- Task editing.
- Settings.
- Progress visualization.
- Approval dialogs.

### Native Application Layer

**Rust**

Responsibilities:

- Secure file access.
- Obsidian vault parsing.
- SQLite access.
- Model process lifecycle.
- Audio coordination.
- Window management.
- File watching.
- Structured command validation.

### Local LLM

**llama.cpp**

Responsibilities:

- Local text generation.
- Structured task extraction.
- Summaries.
- Follow-up questions.
- Goal decomposition.
- Review drafting.

Repository:

- <https://github.com/ggml-org/llama.cpp>

The application should support configurable GGUF models rather than hard-coding one model.

A Qwen3-family model can be considered as an initial default because Qwen3 open-weight models are distributed under Apache 2.0:

- <https://github.com/QwenLM/Qwen3>

### Speech Recognition

**whisper.cpp**

Responsibilities:

- Local speech-to-text.
- Voice activity detection.
- Microphone transcription.
- Optional streaming transcription.

Repository:

- <https://github.com/ggml-org/whisper.cpp>

### Text-to-Speech

**Sherpa-ONNX**

Responsibilities:

- Offline voice synthesis.
- Cross-platform local voice playback.
- Voice selection.
- Potential streaming output.

Repository:

- <https://github.com/k2-fsa/sherpa-onnx>

### Storage

**SQLite**

Responsibilities:

- Task hierarchy.
- Long-term goals held before a vault is connected.
- Declared sections, including empty ones (§6.2).
- Daily plans.
- Weekly plans.
- Monthly plans.
- Sticky-note layouts.
- App settings.
- Obsidian source mappings.
- Conversation metadata.
- Rollover history.

Obsidian remains the permanent long-term knowledge source once it is connected (§3.2).

---

# 9. Obsidian Integration

This section describes an integration that arrives at the end of the roadmap, after the boards, the standup, voice, and the reviews are all working (§22). Nothing described here is required for those features to function; the vault deepens them rather than enabling them.

## 9.1 Supported Markdown Information

The application should initially parse:

- YAML frontmatter.
- Note titles.
- Headings.
- Markdown checkboxes.
- Wikilinks.
- Parent relationships.
- Tags.
- Status values.
- Priority values.
- Due dates when present.
- Last-updated dates.
- Callouts and summaries when useful.

Example source note:

```yaml
---
parent: "[[Projects]]"
status: Ongoing
priority: 8
last_updated: 2026-08-20
---
```

Example task:

```markdown
- [ ] Schedule a dental cleaning and routine checkup.
```

## 9.2 Read Strategy

The application should not send the entire vault to the LLM.

Instead:

1. Parse Markdown deterministically.
2. Store a lightweight local index.
3. Identify active projects and unchecked tasks.
4. Rank candidates using priority, status, due date, and recency.
5. Retrieve a small set of relevant note sections.
6. Show their sources in the interface.
7. Pass only those excerpts to the LLM.

## 9.3 Initial Retrieval Rules

The first version does not require embeddings.

A deterministic ranking system can use:

```text
Task relevance score =
    project priority
  + due-date urgency
  + active-status weight
  + weekly/monthly connection
  + rollover count
  + recent user mentions
```

Semantic embeddings can be added later for large or less structured vaults.

## 9.4 Source Transparency

Every AI recommendation based on Obsidian should expose its source.

Example:

```text
Schedule dental appointment
Source: Health/Medical/Dental.md
Reason: Priority 8, not started, unchecked next action
```

## 9.5 Write Strategy

The safest default is to write planning artifacts into dedicated folders:

```text
Daily Standups/
Weekly Reviews/
Monthly Reviews/
```

The application should avoid rewriting original project notes unless the user explicitly approves the exact change.

## 9.6 Proposed Metadata Improvements

When information is missing, the assistant may propose optional fields:

```yaml
next_action: Schedule the dental appointment
due: 2026-08-28
review: weekly
desired_outcome: Complete routine dental checkup
```

The assistant should ask rather than inventing these values.

---

# 10. Task Data Model

A task should have a stable internal identifier independent of its displayed text.

```typescript
type TaskHorizon = "daily" | "weekly" | "monthly" | "long-term";

type TaskStatus =
  | "backlog"
  | "planned"
  | "in-progress"
  | "blocked"
  | "completed"
  | "cancelled"
  | "deferred";

interface Task {
  id: string;
  title: string;
  description?: string;

  horizon: TaskHorizon;
  status: TaskStatus;

  parentTaskId?: string;
  childTaskIds: string[];

  sourceType: "obsidian" | "standup" | "manual";
  sourceFile?: string;
  sourceLine?: number;

  area?: string;
  project?: string;
  priority?: number;

  scheduledDate?: string;
  periodStart?: string;
  periodEnd?: string;
  dueDate?: string;
  completedAt?: string;

  progressCurrent?: number;
  progressTarget?: number;
  progressUnit?: string;

  blocker?: string;
  notes?: string;

  /** How long the task actually took, in minutes. Null until recorded. */
  timeSpentMinutes?: number;

  rolloverCount: number;
  createdAt: string;
  updatedAt: string;
}
```

`area` and `project` are the section names described in §6.2 — `area` on the Priority board, `project` on the Weekly board. A section the user has declared exists whether or not any task currently carries its name, so an empty section is not represented by these fields alone; a task with neither field set is Unsorted.

## 10.1 Parent-Child Relationships

Examples:

```text
Monthly task
└── Weekly task
    ├── Daily task
    ├── Daily task
    └── Daily task
```

Completing a child task should update progress but should not automatically mark the parent complete unless its completion rule has been satisfied.

## 10.2 Progress Types

Supported progress types may include:

- Completed subtasks.
- Numeric target.
- Percentage.
- Binary complete/incomplete.
- Manually reported progress.

Example:

```text
Submit job applications
Current: 12
Target: 20
Unit: applications
```

## 10.3 Rollover Tracking

Every time a task is rescheduled, its rollover count increases.

The assistant can use this information during reflection:

> You have moved this task five times. Is it blocked, too large, no longer important, or emotionally difficult to start?

The application should avoid using judgmental language.

## 10.4 Recording How Long Work Took

Tasks record how long they actually took, so a week can be totalled at the end of it. Estimates are guesses; a recorded duration is evidence, and it is the only honest basis for the question "was that week realistic?"

### A logged duration, not a stopwatch

The user types or picks how long something took — usually when marking it done. The application does not run a live timer.

This is deliberate. A stopwatch demands that the user remember to start it, remember to stop it, and work in uninterrupted blocks. In practice it produces a mixture of forgotten starts and timers left running overnight, and every total built on it becomes untrustworthy. A duration entered from memory is approximate, but it is approximate in a way the user knows about.

One number per task is enough because a task belongs to a single day. Summing a period means summing the tasks scheduled inside it — there is no work spanning a week boundary that needs splitting.

### Rules

- **Duration is optional.** Completing a task must never be blocked by a request for a number. A task with no recorded time simply contributes nothing to the total.
- **Recording is one gesture.** Common values are offered as presets — 15m, 30m, 1h, 2h — with a free-text field for anything else. If logging takes longer than a few seconds, it stops happening.
- **A duration can be edited at any time**, not only at completion.
- **Time for a period** is the sum of `timeSpentMinutes` across tasks scheduled in that period.
- **Implausible values are questioned, not rejected.** A task logged at eighteen hours is more likely a typo than a marathon, and is worth surfacing during the evening check-in — but the user may be right, so the application asks rather than refusing.

### What this enables

- A weekly recap of hours spent, broken down by area and project (§13.2).
- Comparing planned capacity against time actually spent (§11.3).
- Noticing that a task deferred five times has no recorded time at all, which says something different from one deferred five times with six hours behind it.

---

# 11. Standup Conversation Design

## 11.1 Session Stages

A daily standup should follow controlled stages:

```text
1. Context
2. Previous progress
3. Current priorities
4. Blockers
5. Capacity
6. Proposed commitments
7. User approval
8. Save and close
```

## 11.2 Context Stage

The assistant may summarize:

- Current weekly commitments.
- Yesterday's incomplete tasks.
- Upcoming due dates.
- High-priority sections and projects on the boards.
- Repeatedly deferred tasks.
- Monthly progress.
- High-priority Obsidian projects, once a vault is connected.

Everything but the last item comes from the application's own database, which is why the standup is useful before any vault exists.

## 11.3 Capacity Check

Before proposing work, the assistant should ask about available time.

Example:

> How much focused time do you realistically have today?

This helps prevent generating an unrealistic plan.

Once a few weeks of recorded durations exist, the assistant can compare the answer against what similar work actually took, rather than accepting an estimate at face value.

## 11.4 Task Proposal

The assistant should generally recommend:

- One primary task.
- One or two secondary tasks.
- Optional small administrative tasks.

It should explain how each task connects to a weekly or monthly outcome.

## 11.5 Structured Output

The LLM should return structured data validated by application code.

The example below shows the fullest form, with a vault connected. Before that, `sourceFile` is absent and `proposedObsidianWrites` is always empty; a proposal that names a `section` instead is equally valid. The schema does not change when Obsidian arrives — those fields simply start being populated.

Example:

```json
{
  "summary": "Focus on job search and one health task.",
  "tasks": [
    {
      "title": "Customize résumé and apply to Company X",
      "horizon": "daily",
      "parentTaskId": "weekly-job-applications",
      "estimatedMinutes": 60,
      "sourceFile": "Job Search/Fall 2026.md"
    },
    {
      "title": "Call the dental clinic",
      "horizon": "daily",
      "estimatedMinutes": 15,
      "sourceFile": "Health/Medical/Dental.md"
    }
  ],
  "blockers": [],
  "proposedObsidianWrites": []
}
```

Invalid output must be rejected or repaired before being saved.

---

# 12. Voice Experience

## 12.1 Initial Interaction Model

The MVP should use push-to-talk or microphone toggle.

This is preferable to always listening because it:

- Protects privacy.
- Reduces accidental recording.
- Reduces resource use.
- Simplifies voice activity detection.
- Avoids requiring a wake-word engine.

## 12.2 Speech Pipeline

```text
Microphone
    ↓
Voice activity detection
    ↓
whisper.cpp transcription
    ↓
Conversation controller
    ↓
Relevant Obsidian context
    ↓
llama.cpp response
    ↓
Sentence segmentation
    ↓
Sherpa-ONNX speech synthesis
    ↓
Audio playback
```

## 12.3 Talkback Behavior

Responses should be conversational but concise.

The assistant should avoid reading long task lists aloud. It should summarize and let the visual interface show details.

Example:

> I suggest three commitments today: one application, the dental call, and an hour on the standup project. Does that feel realistic?

## 12.4 Interruption

A later version may allow the user to interrupt the assistant while it is speaking.

The MVP can use a simpler Stop Speaking button.

## 12.5 Voice Recording Policy

By default:

- Process audio in memory.
- Delete temporary audio after transcription.
- Save transcripts only if enabled.
- Never save raw audio unless explicitly requested.

---

# 13. Daily, Weekly, and Monthly Notes

These notes are produced from the first release onward and saved locally, and can be exported as Markdown files. They are written into an Obsidian vault only once writeback ships (§22, Phase 6); the wikilinks in the Connections sections below appear only when there is a vault for them to point into.

Notes identify a week by its ISO form — `2026-W34` — in frontmatter, in filenames, and in note titles. This is deliberate and is not what the boards show. A note is a stored artifact that will be sorted, searched, linked, and matched against other notes years later, and for that an unambiguous identifier that sorts correctly as plain text is worth more than a readable one. The Weekly Progress board has the opposite job and uses the readable form instead (§6.5).

## 13.1 Daily Standup Note

Suggested format:

```markdown
---
date: 2026-08-20
type: daily-standup
week: 2026-W34
month: 2026-08
---

# Daily Standup — August 20, 2026

## Capacity

- Available focused time: 3 hours
- Energy: Moderate

## Today

- [ ] Customize résumé and apply to Company X
- [ ] Call the dental clinic
- [ ] Write the MVP architecture section

## Connections

- Company X application → [[Fall 2026]]
- Dental call → [[Dental]]
- MVP architecture → [[AI Daily Assistant]]

## Blockers

- Need to decide which model size to support initially.

## Notes

- Keep the scope focused on Windows for the first prototype.

## Evening Reflection

- Completed:
- Deferred:
- Learned:
```

## 13.2 Weekly Review Note

```markdown
---
type: weekly-review
week: 2026-W34
---

# Weekly Review — 2026-W34

## Summary

- Planned tasks: 14
- Completed tasks: 10
- Completion rate: 71%
- Tasks carried forward: 3
- Cancelled tasks: 1
- Time tracked: 18h 20m across 5 days

## Time by Area

| Area              | Tracked | Share |
| ----------------- | ------- | ----- |
| Job Search        | 7h 10m  | 39%   |
| AI Daily Assistant| 9h 45m  | 53%   |
| Health            | 1h 25m  | 8%    |

## Progress by Area

### Job Search

- Completed three applications.
- Weekly target was five.
- 7h 10m tracked, averaging 2h 23m per application.

### Health

- Dental appointment remains unscheduled.
- Task has been moved three times, with no time recorded against it.

### AI Daily Assistant

- Product specification completed.
- Prototype not started.
- 9h 45m tracked, the largest share of the week.

## Blockers

- Underestimated application preparation time.
- Avoided making the dental call.

## Next Week

- [ ] Submit five applications
- [ ] Schedule dental appointment
- [ ] Build sticky-note window prototype
```

## 13.3 Monthly Review Note

```markdown
---
type: monthly-review
month: 2026-08
---

# Monthly Review — August 2026

## Commitments

### Job Search

- Target: 20 applications
- Completed: 16
- Result: Partially completed

### AI Daily Assistant

- Target: Complete MVP design and prototype
- Result: Design completed; prototype in progress

### Health

- Target: Schedule three appointments
- Completed: 2
- Result: Partially completed

## What Worked

- Daily plans were most effective when limited to three priorities.

## Recurring Blockers

- Administrative phone calls were repeatedly deferred.
- Large technical tasks needed smaller daily actions.

## Priorities for Next Month

- Complete and release the first desktop prototype.
- Continue job-search consistency.
- Finish remaining health appointment.
```

---

# 14. Lightweight Mode

The application must remain useful without loading AI.

Lightweight mode should support:

- Viewing boards.
- Completing tasks.
- Editing task text.
- Recording how long a task took.
- Setting a task's priority.
- Creating, renaming, and deleting sections.
- Expanding and collapsing days and sections.
- Moving tasks between dates and between sections.
- Adding tasks.
- Opening linked Obsidian notes, once a vault is connected.
- Viewing progress.
- Changing board positions.
- Receiving reminders.
- Reviewing previously generated summaries.

The following actions may start AI after confirmation:

- Start Daily Standup.
- Start Evening Reflection.
- Plan My Week.
- Monthly Review.
- Break This Task Down.
- Help Me Resolve This Blocker.
- Summarize Progress.
- Suggest Priorities.

---

# 15. Performance Expectations

Actual performance depends on hardware and model selection.

A computer with 16 GB of RAM should generally be able to run:

- A quantized 3B–8B local language model.
- A small or base Whisper model.
- A lightweight offline TTS model.
- The desktop application.

Expected behavior:

- Sticky-note mode uses relatively little memory.
- LLM mode may use several gigabytes of RAM or VRAM.
- Model loading may take several seconds.
- Speech recognition may operate near real time.
- A generated response may take approximately 2–10 seconds depending on hardware.
- All model memory should be released after inference processes terminate.

The installer should offer hardware-based profiles:

```text
Lightweight
- Smaller LLM
- Whisper tiny/base
- Faster startup
- Lower memory use

Balanced
- Mid-sized quantized LLM
- Whisper base/small
- Better planning quality

High Quality
- Larger local model
- Larger Whisper model
- Higher memory and GPU requirements
```

---

# 16. Privacy and Security

## 16.1 Default Privacy Rules

- No account required.
- No cloud API required.
- No telemetry by default.
- No remote logging.
- No automatic vault modification.
- No raw audio retention by default.
- No model prompt retention unless enabled.
- No background microphone access outside an active session.

## 16.2 Vault Permissions

The user should be able to configure:

- Included folders.
- Excluded folders.
- Read-only mode.
- Approved write folders.
- Whether original tasks may be updated.
- Whether transcripts may be stored.
- Whether the app may create new notes.

## 16.3 Process Security

Sidecar binaries should:

- Bind only to localhost when using HTTP.
- Use randomly selected local ports.
- Reject remote connections.
- Terminate when the parent session ends.
- Be distributed with checksums.
- Never receive unrestricted file-system access.

## 16.4 Sensitive Notes

The application should support exclusions such as:

```text
Private/
Journal/
Medical/
Financial/
```

Excluded notes must not be indexed, searched, or included in model prompts.

---

# 17. Reliability and Failure Handling

## 17.1 Model Fails to Start

The app should:

- Show a clear error.
- Leave all existing tasks intact.
- Offer diagnostic information.
- Allow retrying with a smaller model.
- Continue supporting manual sticky-note functionality.

## 17.2 Speech Recognition Fails

The user should be able to type instead.

Voice must enhance the product rather than become a requirement.

## 17.3 Obsidian File Changes Externally

The app should:

- Detect changed files.
- Refresh its index.
- Avoid overwriting newer content.
- Show a conflict if an approved write targets a changed note.
- Preserve both versions when necessary.

## 17.4 Application Crashes

Because tasks are persisted before model shutdown:

- Sticky notes should restore after restart.
- Approved plans should not be lost.
- Incomplete conversation drafts may be recoverable.
- Orphaned AI processes should be detected and terminated.

## 17.5 Invalid LLM Output

All structured output should be validated.

The application should never execute file operations directly from unvalidated model text.

---

# 18. Settings

Suggested settings categories:

## General

- Launch at login.
- Show tray icon.
- Start with boards visible.
- Default planning view.
- Week start day.
- Time and date format.

## Obsidian

This category is where a vault is connected, and it appears only once the Obsidian phases ship (§22). Onboarding does not ask for a vault (§5.1).

- Vault path.
- Included folders.
- Excluded folders.
- Read-only mode.
- Daily standup folder.
- Weekly review folder.
- Monthly review folder.
- Write approval requirements.

## AI

- LLM model.
- Context size.
- Inference backend.
- CPU thread count.
- GPU acceleration.
- Idle shutdown timeout.
- Maximum response length.

## Voice

- Microphone.
- Whisper model.
- Language.
- Push-to-talk shortcut.
- TTS voice.
- Voice speed.
- Auto-play responses.
- Save transcripts.
- Save raw audio.

## Sticky Notes

- Theme.
- Accent color.
- Opacity.
- Font size.
- Always-on-top behavior.
- Click-through behavior.
- Default boards.
- Monitor assignment.
- Completed-task appearance.

## Sections

- Whether to hide empty sections after a number of days.
- Whether to show the Unsorted group when it is empty.
- Confirmation before deleting a section.

## Time Logging

- Whether to prompt for a duration when a task is completed.
- Duration presets offered (default 15m, 30m, 1h, 2h).
- Threshold above which a logged duration is queried as a likely typo (default 8 hours).
- Whether to show recorded time on the boards.

## Planning

- Maximum recommended daily tasks.
- Weekly planning day.
- Monthly planning date.
- Morning reminder time.
- Evening reminder time.
- Rollover warning threshold.

---

# 19. Open-Source Strategy

## 19.1 Application License

Potential choices:

- MIT for maximum adoption and simplicity.
- Apache 2.0 for explicit patent protections.
- AGPL if modified hosted versions should also remain open.

MIT or Apache 2.0 would likely be the most approachable for a desktop application.

**Decision (2026-08-20): MIT.** It is the shortest and most permissive option, has the
highest adoption for desktop applications, and is compatible with llama.cpp (MIT),
whisper.cpp (MIT), Sherpa-ONNX (Apache 2.0), and Qwen3 weights (Apache 2.0).

## 19.2 Model Licensing

The application code and downloaded models should be treated separately.

The repository should not necessarily include large model files.

Instead, the setup experience should:

1. Display available models.
2. Show the model size.
3. Show the license.
4. Explain hardware requirements.
5. Download the selected model after approval.
6. Store it in the application's local model directory.

## 19.3 Contributor Opportunities

Community contributions could include:

- Additional local model adapters.
- New TTS voices.
- Themes.
- Obsidian parsing improvements.
- Other Markdown vault adapters.
- Localization.
- Accessibility improvements.
- Hardware-specific performance optimizations.
- Alternative planning methodologies.
- Plugin or extension APIs.

---

# 20. Suggested Repository Structure

```text
local-standup/
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── SECURITY.md
├── CODE_OF_CONDUCT.md
├── package.json
├── src/
│   ├── app/
│   ├── components/
│   ├── features/
│   │   ├── boards/
│   │   ├── standup/
│   │   ├── weekly-review/
│   │   ├── monthly-review/
│   │   ├── settings/
│   │   └── onboarding/
│   ├── stores/
│   └── types/
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   ├── inference/
│   │   ├── obsidian/
│   │   ├── storage/
│   │   ├── audio/
│   │   └── windows/
│   ├── migrations/
│   ├── capabilities/
│   ├── binaries/
│   └── tauri.conf.json
├── prompts/
│   ├── daily-standup.md
│   ├── evening-review.md
│   ├── weekly-planning.md
│   └── monthly-review.md
├── docs/
│   ├── architecture.md
│   ├── privacy.md
│   ├── model-support.md
│   └── obsidian-format.md
└── tests/
    ├── fixtures/
    ├── integration/
    └── end-to-end/
```

---

# 21. MVP Scope

The first release should be deliberately focused.

## 21.1 Included

- Windows-first Tauri desktop application.
- Manual task creation.
- Daily, weekly, and monthly task types.
- Long-term goals and monthly commitments held in the application itself.
- Parent-child task relationships.
- Priority Tasks board.
- Weekly Tasks board.
- Weekly Progress board.
- Monthly Progress board.
- User-declared sections on the Priority and Weekly boards (§6.2).
- Renaming and deleting sections, with deletion unfiling rather than destroying.
- Priority settable from 1 to 10 on any task row, with boards re-sorting on change.
- Recording how long each task took.
- Expandable Monday-to-Sunday weekly view.
- Priority shown on every task row.
- Movable and resizable sticky windows.
- Persistent window positions.
- System-tray controls.
- Push-to-talk standup.
- Local speech transcription.
- Local LLM conversation.
- Local text-to-speech.
- Proposed daily plan with user approval.
- Daily standup summaries saved locally.
- Complete model shutdown after the session.
- Manual operation when AI is unavailable.

## 21.2 Excluded From the First MVP

Obsidian is excluded from the first release. Specifically:

- Selecting an Obsidian vault.
- Parsing YAML frontmatter and Markdown checkboxes.
- Reading status and priority out of notes.
- Identifying active projects and unchecked tasks in a vault.
- Writing daily standup, weekly review, or monthly review notes into a vault.
- Folder exclusions, source links, and file watching.

The reason is that the standup does not need the vault in order to be useful. The application now holds its own long-term goals, its own monthly and weekly commitments, and its own named sections (§6.2), which is everything a standup needs to ask a good question: what did you say you would do, what is still open, and what is worth doing today.

Putting the vault first inverted that. It meant nothing worked until parsing somebody else's Markdown worked — an open-ended problem, because every vault is organised differently (§24) — and until it did, there was no planner to show anyone. The first release should be a planner that happens to have no vault yet, not a vault reader that cannot yet plan.

Obsidian remains the long-term source of truth (§3.2) and the last two roadmap phases are dedicated to it (§22). It is deferred, not dropped.

Also excluded:

- Always-listening wake word.
- Cloud synchronization.
- Mobile application.
- Multi-user collaboration.
- Email and calendar integrations.
- Autonomous modification of arbitrary notes.
- Fully autonomous scheduling.
- Complex vector search.
- Voice cloning.
- Emotion recognition.
- Continuous background LLM operation.
- macOS and Linux installers unless development capacity allows.

---

# 22. Development Roadmap

The phases are ordered so that every one of them ends with something a person can use. The application becomes a planner, then a planner that talks, then a planner that talks and reflects, and only then does it learn to read and write an Obsidian vault. Each phase adds a capability to a product that already worked without it.

Obsidian comes last for that reason. It is the only phase whose difficulty depends on other people's files, so it is the one most likely to overrun — and putting it early would mean the overrun happened while there was still nothing to ship (§21.2).

## Phase 1: Lightweight Planning Boards

Build the useful non-AI foundation:

- SQLite task database.
- Daily, weekly, and monthly task hierarchy.
- Long-term goals and monthly commitments stored in the application.
- Priority Tasks board.
- Weekly Tasks board.
- Weekly Progress board.
- Monthly board.
- User-declared sections, with rename and unfiling delete (§6.2).
- Priority editing from the task badge, with re-sorting.
- Window persistence.
- Tray controls.
- Manual task editing.

Success means the application is already a functional desktop planner.

## Phase 2: Local Text Standup

Add:

- `llama.cpp` process manager.
- Model setup.
- Typed standup conversation.
- Context selection from the application's own goals, commitments, sections, and open tasks.
- Structured task proposals.
- Approval workflow.
- Model shutdown and resource verification.

The context the assistant needs already exists after Phase 1: the boards hold the commitments, the sections name the areas of work, and the rollover counts record what keeps being deferred. No vault is required to hold a useful standup.

Success means the user can plan the day using a local text conversation.

## Phase 3: Local Voice

Add:

- Microphone recording.
- `whisper.cpp`.
- Voice activity detection.
- Sherpa-ONNX text-to-speech.
- Push-to-talk.
- Voice selection.
- Temporary audio cleanup.

Success means the complete standup can be performed by voice.

## Phase 4: Reviews

Add:

- Evening reflection.
- Weekly planning.
- Weekly retrospective.
- Monthly planning.
- Monthly retrospective.
- Review notes saved locally, and exportable as Markdown files.
- Time and completion summaries by area and project.

Reviews are built before the vault because the material they review is the application's own: planned versus completed tasks, rollover counts, and recorded durations. Writing them out to Obsidian is a destination, and the destination can be added later without changing what a review says.

Success means the full daily, weekly, and monthly cycle runs end to end.

## Phase 5: Obsidian Read Integration

Add:

- Vault selection.
- Markdown scanning.
- Frontmatter parsing.
- Checkbox extraction.
- Wikilink relationships.
- Source links.
- Folder exclusions.
- File watching.

By this point there is a working planner for vault tasks to be promoted *into*, and a working standup for vault context to be cited *in*. Reading is deliberately shipped on its own, ahead of any writing, so that the risky half of the integration is proven while the vault is still strictly read-only (§9.2).

Success means tasks can be promoted from Obsidian into planning boards, and standups can cite vault sources.

## Phase 6: Obsidian Writeback

Add:

- Dedicated Obsidian daily standup, weekly review, and monthly review notes.
- Controlled task updates in original notes.
- Diff and approval interface.
- Conflict detection against externally changed files.

This is the last functional phase because it is the only one that modifies files the application did not create. Every safeguard it depends on — approval before writing (§3.4), source transparency (§9.4), append-only review folders (§9.5) — is easier to get right once the reviews being written already exist and are known to be correct.

Success means planning artifacts reach the vault, and no note is ever changed without the user approving the exact diff.

## Phase 7: Cross-Platform and Community Release

Add:

- macOS support.
- Linux support.
- Signed installers.
- Automated releases.
- Contributor documentation.
- Model adapter interface.
- Theme and prompt customization.
- Accessibility review.

---

# 23. MVP Acceptance Criteria

The MVP is successful when all of the following are true:

## Desktop Boards

- The user can create daily, weekly, and monthly tasks.
- Boards remain visible after the main window closes.
- Boards restore their positions after restarting.
- Completing a task updates progress immediately.
- Each task row shows its priority, completion state, and recorded time.
- The weekly board lists Monday to Sunday, and days expand and collapse.
- The weekly board labels its week as a readable range and number, such as `Aug 24-30 - Week 35`.
- A task can be completed without recording a duration.
- Recorded time survives a restart.
- Board interactions, including recording time, do not load the LLM.

## Sections

- The user can declare a named section on the Priority and Weekly boards.
- A newly created section stays visible while empty, and sorts above sections that contain work.
- Renaming a section moves every task in it, with no state in which the group appears twice.
- Deleting a section leaves its tasks in Unsorted rather than deleting them.
- "Job Search" and `job search ` resolve to the same section.
- Changing a task's priority re-sorts the board, and can move its section.

## AI

- The user can start a standup manually.
- The local model starts only when needed.
- The assistant's context comes from the user's own goals, commitments, sections, and open tasks, and the interface shows what it used.
- The assistant proposes structured tasks.
- The user can edit or reject every proposal.
- The model process terminates after the session.
- RAM and GPU memory are released.

## Voice

- The user can record a standup response.
- Speech is transcribed locally.
- The assistant's response can be spoken locally.
- Raw recordings are deleted by default.
- Typing remains available as a fallback.

## Reviews

- Daily completion contributes to weekly progress.
- Weekly progress contributes to monthly progress.
- Rollover counts are preserved.
- Weekly and monthly summaries can be saved as Markdown files.
- The weekly review reports hours tracked, broken down by area.

## Obsidian (Post-MVP)

None of the following is required for the MVP to be considered successful. These are the acceptance criteria for the Obsidian phases (§22, Phases 5 and 6), listed here so the standard they will be held to is agreed in advance:

- The user can select a vault.
- The app identifies Markdown tasks and frontmatter.
- Every promoted task retains its source link.
- Excluded folders are never indexed.
- Vault-derived context appears in the assistant's summary with its sources.
- No original note is changed without approval of the exact diff.

---

# 24. Risks and Mitigations

## Local Models May Be Slow

**Risk:** Lower-powered computers may produce slow responses.

**Mitigation:**

- Hardware detection.
- Multiple model profiles.
- Quantized models.
- Short responses.
- Limited retrieved context.
- Clear loading indicators.

## Model Quality May Be Inconsistent

**Risk:** Small local models may misunderstand complicated notes.

**Mitigation:**

- Deterministic parsing.
- Structured prompts.
- Source citations.
- User approval.
- Schema validation.
- Avoid relying on AI for calculations.

## Vaults May Be Messy

**Risk:** Different users organize Obsidian differently.

**Mitigation:**

- Ship the planner before the vault reader, so an overrun here delays an enhancement rather than the product (§22).
- Configurable mappings.
- Folder exclusions.
- Preview discovered structure.
- Start with common Markdown conventions.
- Add adapters later.

## Sticky Notes May Become Cluttered

**Risk:** Too many boards or tasks may overwhelm the desktop.

**Mitigation:**

- Daily task limits.
- Collapsible boards.
- Separate horizons.
- Focus mode.
- Automatic archiving.
- Manual board visibility.

## Repeated Planning Could Become Annoying

**Risk:** The assistant may ask too many questions.

**Mitigation:**

- Configurable session length.
- Skip buttons.
- Concise voice responses.
- Quick planning mode.
- Remember stable preferences locally.

## Obsidian Conflicts

**Risk:** The user may edit a note while the app prepares a change.

**Mitigation:**

- File modification checks.
- Diff preview.
- Conflict warnings.
- Append-only review notes by default.
- Backups before modifying existing notes.

---

# 25. Product Boundaries

The application is:

- A personal planning assistant.
- A local voice interface.
- A bridge between long-term goals and current actions.
- A structured desktop task display.
- A reflection and review tool.

The application is not:

- A replacement for Obsidian.
- An always-listening surveillance assistant.
- A fully autonomous life manager.
- A cloud-dependent AI service.
- A system that should make important decisions without the user.
- A traditional chatbot that happens to display tasks.

---

# 26. Key Product Decision

The foundational architectural requirement is:

> The planning boards are always available, but AI inference is always on demand.

The visible sticky notes belong to a lightweight desktop host. The local LLM, speech recognition engine, and voice engine are separate processes that start only during a standup, planning session, reflection, or explicit AI request.

This provides the visibility of persistent desktop sticky notes without wasting several gigabytes of RAM or GPU memory throughout the day.

---

# 27. Short Product Description

A private, open-source desktop planning companion that conducts local voice standups, turns long-term priorities into monthly, weekly, and daily commitments, keeps those commitments visible as lightweight desktop sticky notes, and can read the goals you already keep in Obsidian.

---

# 28. One-Sentence Pitch

Turn your long-term goals — kept in the app, or in your Obsidian vault once you connect it — into realistic monthly commitments, weekly milestones, and daily actions through private voice standups that run entirely on your computer.

---

# 29. Possible Project Names

- SoloStand
- Local Standup
- Standup Desktop
- Dayboard
- Goalboard
- Anchor
- Northstar
- Daily Thread
- Local Compass
- Self Standup

A final project name should be checked for existing trademarks, domains, package names, and GitHub repositories before adoption.

**Working name (2026-08-20): My Daily Standup.** Not yet checked for trademark or package-name
conflicts; revisit before the first public release.
