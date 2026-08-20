---
title: Local AI Daily Standup and Sticky-Note Planner
type: Product and Technical Specification
status: Concept
version: 0.1
last_updated: 2026-08-20
license: MIT
---

# Local AI Daily Standup and Sticky-Note Planner

## Executive Summary

This project is an open-source, local-first desktop application that helps a person plan and review their life through private voice conversations.

The application connects to an Obsidian vault containing long-term goals, projects, priorities, and tasks. It uses that information to conduct personal daily standups, weekly planning sessions, monthly planning sessions, and retrospective reviews.

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

---

# 1. Product Vision

Create a private personal planning companion that turns long-term goals into actions that can realistically be completed today.

The application should feel like having a short standup with an organized version of yourself. It should:

- Remember what matters through Obsidian.
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

Long-term goals live in Obsidian.

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

Completing a daily task contributes to the weekly milestone. Weekly progress contributes to the monthly commitment. Monthly reviews summarize progress toward the long-term Obsidian goal.

---

# 5. Primary User Experience

## 5.1 First-Time Setup

During onboarding, the application should:

1. Explain that all AI processing is local.
2. Ask the user to select an Obsidian vault.
3. Request read-only access initially.
4. Scan the vault's Markdown structure.
5. Explain which files and task patterns it discovered.
6. Let the user exclude private folders.
7. Detect available CPU, RAM, and GPU capabilities.
8. Recommend suitable local models.
9. Download models only after confirmation.
10. Test the microphone and selected voice.
11. Ask where sticky-note boards should appear.
12. Offer to start with daily planning only or enable all planning horizons.

## 5.2 Morning Standup

At a configurable time, the application displays a notification:

> Ready for your morning standup?

When the user starts the session:

1. The planning window opens.
2. Whisper and the selected LLM are launched.
3. Relevant Obsidian context is retrieved.
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
8. Save a weekly note in Obsidian.
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
8. Save the monthly plan and retrospective in Obsidian.

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

## 6.2 Priority Tasks Board

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

Priority tasks may originate from Obsidian or be promoted manually.

## 6.3 Weekly Tasks Board

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

## 6.4 Weekly Progress Board

The Weekly Progress board shows completed and planned work organized by day.

Example:

```text
WEEKLY PROGRESS

Monday
✓ Submit application to Company A
✓ Draft project specification
○ Email professor

Tuesday
✓ Complete onboarding task
○ Schedule dental appointment

Wednesday
○ Apply to Company B
○ Compare hotels
```

Completed items should remain visible but become dimmed or crossed out. This provides a record of the week rather than making completed work disappear.

## 6.5 Monthly Board

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

## 6.6 Sticky-Note Interactions

Users should be able to:

- Complete a task.
- Uncomplete a task.
- Edit task text.
- Move a task to another day.
- Promote a daily task to weekly.
- Move a weekly task to another week.
- Link a task to an Obsidian note.
- Open the source note in Obsidian.
- Add a blocker.
- Add a short comment.
- Delete or archive a task.
- Ask the AI for help when explicitly requested.

Normal task operations must not start the LLM.

## 6.7 Window Behaviors

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

## 6.8 Tray Menu

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
- Daily plans.
- Weekly plans.
- Monthly plans.
- Sticky-note layouts.
- App settings.
- Obsidian source mappings.
- Conversation metadata.
- Rollover history.

Obsidian remains the permanent long-term knowledge source.

---

# 9. Obsidian Integration

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

  rolloverCount: number;
  createdAt: string;
  updatedAt: string;
}
```

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
- High-priority Obsidian projects.
- Repeatedly deferred tasks.
- Monthly progress.

## 11.3 Capacity Check

Before proposing work, the assistant should ask about available time.

Example:

> How much focused time do you realistically have today?

This helps prevent generating an unrealistic plan.

## 11.4 Task Proposal

The assistant should generally recommend:

- One primary task.
- One or two secondary tasks.
- Optional small administrative tasks.

It should explain how each task connects to a weekly or monthly outcome.

## 11.5 Structured Output

The LLM should return structured data validated by application code.

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

## Progress by Area

### Job Search

- Completed three applications.
- Weekly target was five.

### Health

- Dental appointment remains unscheduled.
- Task has been moved three times.

### AI Daily Assistant

- Product specification completed.
- Prototype not started.

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
- Moving tasks between dates.
- Adding tasks.
- Opening linked Obsidian notes.
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
- Select an Obsidian vault.
- Parse YAML frontmatter and Markdown checkboxes.
- Read status and priority.
- Identify active projects and unchecked tasks.
- Manual task creation.
- Daily, weekly, and monthly task types.
- Parent-child task relationships.
- Priority Tasks board.
- Weekly Tasks board.
- Weekly Progress board.
- Monthly Progress board.
- Movable and resizable sticky windows.
- Persistent window positions.
- System-tray controls.
- Push-to-talk standup.
- Local speech transcription.
- Local LLM conversation.
- Local text-to-speech.
- Proposed daily plan with user approval.
- Dedicated daily standup notes.
- Complete model shutdown after the session.
- Manual operation when AI is unavailable.

## 21.2 Excluded From the First MVP

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

## Phase 1: Lightweight Planning Boards

Build the useful non-AI foundation:

- SQLite task database.
- Daily, weekly, and monthly task hierarchy.
- Priority Tasks board.
- Weekly Tasks board.
- Weekly Progress board.
- Monthly board.
- Window persistence.
- Tray controls.
- Manual task editing.

Success means the application is already a functional desktop planner.

## Phase 2: Obsidian Read Integration

Add:

- Vault selection.
- Markdown scanning.
- Frontmatter parsing.
- Checkbox extraction.
- Wikilink relationships.
- Source links.
- Folder exclusions.
- File watching.

Success means tasks can be promoted from Obsidian into planning boards.

## Phase 3: Local Text Standup

Add:

- `llama.cpp` process manager.
- Model setup.
- Typed standup conversation.
- Context selection.
- Structured task proposals.
- Approval workflow.
- Model shutdown and resource verification.

Success means the user can plan the day using a local text conversation.

## Phase 4: Local Voice

Add:

- Microphone recording.
- `whisper.cpp`.
- Voice activity detection.
- Sherpa-ONNX text-to-speech.
- Push-to-talk.
- Voice selection.
- Temporary audio cleanup.

Success means the complete standup can be performed by voice.

## Phase 5: Reviews and Obsidian Writeback

Add:

- Evening reflection.
- Weekly planning.
- Weekly retrospective.
- Monthly planning.
- Monthly retrospective.
- Dedicated Obsidian review notes.
- Controlled task updates.
- Diff and approval interface.

## Phase 6: Cross-Platform and Community Release

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
- Board interactions do not load the LLM.

## Obsidian

- The user can select a vault.
- The app identifies Markdown tasks and frontmatter.
- Every promoted task retains its source link.
- Excluded folders are never indexed.
- No original note is changed without approval.

## AI

- The user can start a standup manually.
- The local model starts only when needed.
- Relevant Obsidian context appears with sources.
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
- Weekly and monthly summaries can be saved as Markdown.

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

A private, open-source desktop planning companion that reads your Obsidian goals, conducts local voice standups, turns long-term priorities into monthly, weekly, and daily commitments, and keeps those commitments visible as lightweight desktop sticky notes.

---

# 28. One-Sentence Pitch

Turn your long-term Obsidian goals into realistic monthly commitments, weekly milestones, and daily actions through private voice standups that run entirely on your computer.

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
