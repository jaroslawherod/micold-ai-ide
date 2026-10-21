//! The task table behind background tasks and subagent state (research R6, data-model.md
//! *TaskTable*): one entry per `task_id` the CLI's `system` task lines name.

use std::collections::HashMap;

/// What a task runs (FR-020): a shell command (`local_bash`) or a subagent (`local_agent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    Command,
    Subagent,
}

/// A task's state (FR-020, FR-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Running,
    Finished,
    Failed,
    Stopped,
    /// The CLI exited, restarted or ended the turn while it ran, and never reported it.
    EndedWithoutReport,
}

/// One task of the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub task_id: String,
    /// The tool call that started it (FR-021).
    pub tool_use_id: Option<String>,
    pub description: String,
    /// `None` for a task type the CLI reports that is neither a command nor a subagent.
    pub kind: Option<TaskKind>,
    /// A background task (FR-020), rather than a foreground subagent.
    pub background: bool,
    pub state: TaskState,
    /// The report's summary, when the CLI reported one.
    pub outcome: Option<String>,
    /// The latest progress the CLI reported.
    pub activity: Option<String>,
}

impl Task {
    /// The state to show: `Running` reads as `EndedWithoutReport` while the session's CLI is not
    /// running (FR-022). A view-time overlay; the table is unchanged (FR-023).
    pub fn display_state(&self, cli_running: bool) -> TaskState {
        match self.state {
            TaskState::Running if !cli_running => TaskState::EndedWithoutReport,
            state => state,
        }
    }
}

/// A task line, read tolerantly (research R6). A line without a string `task_id`, or a report
/// without a status this table knows, is no event.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum TaskEvent {
    /// `task_started`: a task began, or a resumed one began again.
    Started {
        task_id: String,
        tool_use_id: Option<String>,
        description: Option<String>,
        kind: Option<TaskKind>,
        background: bool,
    },
    /// `task_updated`: a patch of a known task.
    Updated {
        task_id: String,
        state: Option<TaskState>,
        backgrounded: bool,
        description: Option<String>,
    },
    /// `task_progress`: what a running task does now.
    Progress {
        task_id: String,
        activity: Option<String>,
    },
    /// `task_notification`: the CLI's report of how a task ended.
    Notification {
        task_id: String,
        tool_use_id: Option<String>,
        state: TaskState,
        outcome: Option<String>,
    },
}

/// One entry of a `background_tasks_changed` line's live set.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LiveTask {
    pub(super) task_id: String,
    pub(super) description: Option<String>,
    pub(super) kind: Option<TaskKind>,
}

/// `task_type` → kind: `local_bash` is a command, `local_agent` a subagent.
pub(super) fn kind_of(task_type: Option<&str>) -> Option<TaskKind> {
    match task_type? {
        "local_bash" => Some(TaskKind::Command),
        "local_agent" => Some(TaskKind::Subagent),
        _ => None,
    }
}

/// A reported status → state; `None` for a status that ends nothing (or one not known).
pub(super) fn reported_state(status: &str) -> Option<TaskState> {
    match status {
        "completed" => Some(TaskState::Finished),
        "failed" => Some(TaskState::Failed),
        "killed" | "stopped" => Some(TaskState::Stopped),
        _ => None,
    }
}

/// The tasks by `task_id`, in the order first seen.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TaskTable {
    tasks: HashMap<String, Task>,
    order: Vec<String>,
}

impl TaskTable {
    /// Every task, in the order first seen.
    pub(super) fn iter(&self) -> impl Iterator<Item = &Task> {
        self.order.iter().filter_map(|id| self.tasks.get(id))
    }

    /// The task `task_id`, made `Running` with no description when it is new.
    fn entry(&mut self, task_id: &str, background: bool) -> &mut Task {
        if !self.tasks.contains_key(task_id) {
            self.order.push(task_id.to_string());
        }
        self.tasks
            .entry(task_id.to_string())
            .or_insert_with(|| Task {
                task_id: task_id.to_string(),
                tool_use_id: None,
                description: String::new(),
                kind: None,
                background,
                state: TaskState::Running,
                outcome: None,
                activity: None,
            })
    }

    pub(super) fn event(&mut self, event: &TaskEvent) {
        match event {
            TaskEvent::Started {
                task_id,
                tool_use_id,
                description,
                kind,
                background,
            } => {
                let task = self.entry(task_id, *background);
                task.state = TaskState::Running;
                task.outcome = None;
                task.background |= *background;
                if tool_use_id.is_some() {
                    task.tool_use_id.clone_from(tool_use_id);
                }
                if let Some(description) = description {
                    task.description.clone_from(description);
                }
                task.kind = kind.or(task.kind);
            }
            TaskEvent::Updated {
                task_id,
                state,
                backgrounded,
                description,
            } => {
                let Some(task) = self.tasks.get_mut(task_id) else {
                    return;
                };
                task.background |= *backgrounded;
                if let Some(description) = description {
                    task.description.clone_from(description);
                }
                if let (TaskState::Running, Some(state)) = (task.state, state) {
                    task.state = *state;
                }
            }
            TaskEvent::Progress { task_id, activity } => {
                if let (Some(task), Some(_)) = (self.tasks.get_mut(task_id), activity) {
                    task.activity.clone_from(activity);
                }
            }
            TaskEvent::Notification {
                task_id,
                tool_use_id,
                state,
                outcome,
            } => {
                let task = self.entry(task_id, true);
                if task.state == TaskState::Running {
                    task.state = *state;
                }
                if outcome.is_some() {
                    task.outcome.clone_from(outcome);
                }
                if task.tool_use_id.is_none() {
                    task.tool_use_id.clone_from(tool_use_id);
                }
                if task.description.is_empty() {
                    task.description = outcome.clone().unwrap_or_default();
                }
            }
        }
    }

    /// A `background_tasks_changed` line: the CLI's live set. A listed task not seen before is
    /// made; a running background task it leaves out ended without a report.
    pub(super) fn live_set(&mut self, live: &[LiveTask]) {
        for listed in live {
            let task = self.entry(&listed.task_id, true);
            task.background = true;
            if task.description.is_empty() {
                task.description = listed.description.clone().unwrap_or_default();
            }
            task.kind = task.kind.or(listed.kind);
        }
        for task in self.tasks.values_mut() {
            if task.background
                && task.state == TaskState::Running
                && !live.iter().any(|l| l.task_id == task.task_id)
            {
                task.state = TaskState::EndedWithoutReport;
            }
        }
    }

    /// The CLI started or restarted: every running task ended without a report (FR-022).
    pub(super) fn init(&mut self) {
        self.end_running(|_| true);
    }

    /// The turn ended without a result: every running foreground task ended with it (FR-018).
    pub(super) fn turn_ended(&mut self) {
        self.end_running(|task| !task.background);
    }

    fn end_running(&mut self, which: impl Fn(&Task) -> bool) {
        for task in self.tasks.values_mut() {
            if task.state == TaskState::Running && which(task) {
                task.state = TaskState::EndedWithoutReport;
            }
        }
    }
}
