// Copyright (c) 2026 Michael Wroblewski / ShivaCore / A-TownChain-Okosystems. All Rights Reserved.
// Deterministic task lifecycle scheduler. It models orchestration; it does not spawn processes.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl TaskState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled | Self::TimedOut)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub dependencies: Vec<String>,
    pub state: TaskState,
    pub submitted_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    EmptyId,
    EmptyName,
    ZeroTimeout,
    DuplicateTask(String),
    MissingDependency(String),
    DependencyFailed(String),
    TaskNotFound(String),
    InvalidTransition { id: String, state: TaskState },
    InvalidParallelism,
}

#[derive(Debug)]
pub struct TaskScheduler {
    max_parallel: usize,
    tasks: BTreeMap<String, Task>,
}

impl TaskScheduler {
    pub fn new(max_parallel: usize) -> Result<Self, SchedulerError> {
        if max_parallel == 0 {
            return Err(SchedulerError::InvalidParallelism);
        }
        Ok(Self {
            max_parallel,
            tasks: BTreeMap::new(),
        })
    }

    pub fn schedule(
        &mut self,
        id: &str,
        name: &str,
        dependencies: Vec<String>,
        now_ms: u64,
        timeout_ms: u64,
    ) -> Result<(), SchedulerError> {
        let id = id.trim();
        if id.is_empty() {
            return Err(SchedulerError::EmptyId);
        }
        if name.trim().is_empty() {
            return Err(SchedulerError::EmptyName);
        }
        if timeout_ms == 0 {
            return Err(SchedulerError::ZeroTimeout);
        }
        if self.tasks.get(id).is_some() {
            return Err(SchedulerError::DuplicateTask(id.into()));
        }
        let mut seen = std::collections::HashSet::new();
        for dependency in &dependencies {
            if dependency.as_str() == id || !seen.insert(dependency) {
                return Err(SchedulerError::MissingDependency(dependency.clone()));
            }
            let dep = self
                .tasks
                .get(dependency)
                .ok_or_else(|| SchedulerError::MissingDependency(dependency.clone()))?;
            if matches!(
                dep.state,
                TaskState::Failed | TaskState::Cancelled | TaskState::TimedOut
            ) {
                return Err(SchedulerError::DependencyFailed(dependency.clone()));
            }
        }
        self.tasks.insert(
            id.into(),
            Task {
                id: id.into(),
                name: name.trim().into(),
                dependencies,
                state: TaskState::Queued,
                submitted_at_ms: now_ms,
                started_at_ms: None,
                timeout_ms,
            },
        );
        Ok(())
    }

    /// Starts ready tasks in lexical task-ID order, bounded by max_parallel.
    /// A task's timeout clock starts when it begins running.
    pub fn dispatch_ready(&mut self, now_ms: u64) -> Vec<String> {
        self.expire(now_ms);
        // A dependent task must not remain queued forever if a prerequisite failed later.
        let blocked: Vec<String> = self
            .tasks
            .values()
            .filter(|task| {
                task.state == TaskState::Queued
                    && task.dependencies.iter().any(|id| {
                        self.tasks.get(id).is_some_and(|dep| {
                            matches!(
                                dep.state,
                                TaskState::Failed | TaskState::Cancelled | TaskState::TimedOut
                            )
                        })
                    })
            })
            .map(|task| task.id.clone())
            .collect();
        for id in blocked {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.state = TaskState::Failed;
            }
        }
        let running = self
            .tasks
            .values()
            .filter(|task| task.state == TaskState::Running)
            .count();
        let slots = self.max_parallel.saturating_sub(running);
        let ready: Vec<String> = self
            .tasks
            .values()
            .filter(|task| {
                task.state == TaskState::Queued
                    && task.dependencies.iter().all(|id| {
                        self.tasks
                            .get(id)
                            .is_some_and(|dep| dep.state == TaskState::Completed)
                    })
            })
            .map(|task| task.id.clone())
            .take(slots)
            .collect();
        for id in &ready {
            if let Some(task) = self.tasks.get_mut(id) {
                task.state = TaskState::Running;
                task.started_at_ms = Some(now_ms);
            }
        }
        ready
    }

    pub fn complete(&mut self, id: &str) -> Result<(), SchedulerError> {
        self.transition_running(id, TaskState::Completed)
    }

    pub fn fail(&mut self, id: &str) -> Result<(), SchedulerError> {
        self.transition_running(id, TaskState::Failed)
    }

    pub fn cancel(&mut self, id: &str) -> Result<(), SchedulerError> {
        let task = self
            .tasks
            .get_mut(id)
            .ok_or_else(|| SchedulerError::TaskNotFound(id.into()))?;
        if !matches!(task.state, TaskState::Queued | TaskState::Running) {
            return Err(SchedulerError::InvalidTransition { id: id.into(), state: task.state });
        }
        task.state = TaskState::Cancelled;
        Ok(())
    }

    pub fn expire(&mut self, now_ms: u64) -> Vec<String> {
        let mut expired = Vec::new();
        for task in self.tasks.values_mut() {
            if task.state == TaskState::Running
                && task.started_at_ms.is_some_and(|start| {
                    now_ms.saturating_sub(start) >= task.timeout_ms
                })
            {
                task.state = TaskState::TimedOut;
                expired.push(task.id.clone());
            }
        }
        expired
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn running_count(&self) -> usize {
        self.tasks
            .values()
            .filter(|task| task.state == TaskState::Running)
            .count()
    }

    fn transition_running(&mut self, id: &str, target: TaskState) -> Result<(), SchedulerError> {
        let task = self.tasks.get_mut(id).ok_or_else(|| SchedulerError::TaskNotFound(id.into()))?;
        if task.state != TaskState::Running {
            return Err(SchedulerError::InvalidTransition { id: id.into(), state: task.state });
        }
        task.state = target;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependencies_gate_execution_and_parallelism_is_bounded() {
        let mut scheduler = TaskScheduler::new(1).unwrap();
        scheduler.schedule("task:a", "A", vec![], 0, 100).unwrap();
        scheduler.schedule("task:b", "B", vec!["task:a".into()], 0, 100).unwrap();
        assert_eq!(scheduler.dispatch_ready(1), vec!["task:a"]);
        assert!(scheduler.dispatch_ready(2).is_empty());
        scheduler.complete("task:a").unwrap();
        assert_eq!(scheduler.dispatch_ready(3), vec!["task:b"]);
    }

    #[test]
    fn timeout_and_cancel_are_terminal() {
        let mut scheduler = TaskScheduler::new(2).unwrap();
        scheduler.schedule("task:slow", "slow", vec![], 0, 10).unwrap();
        scheduler.schedule("task:cancel", "cancel", vec![], 0, 10).unwrap();
        scheduler.dispatch_ready(5);
        assert_eq!(scheduler.expire(15), vec!["task:cancel", "task:slow"]);
        assert_eq!(scheduler.get("task:slow").unwrap().state, TaskState::TimedOut);
        assert!(scheduler.cancel("task:slow").is_err());

        let mut cancellable = TaskScheduler::new(1).unwrap();
        cancellable.schedule("task:cancel", "cancel", vec![], 0, 10).unwrap();
        assert_eq!(cancellable.cancel("task:cancel"), Ok(()));
        assert_eq!(cancellable.get("task:cancel").unwrap().state, TaskState::Cancelled);
        assert!(cancellable.cancel("task:cancel").is_err());
    }

    #[test]
    fn duplicate_ids_and_missing_dependencies_fail_closed() {
        let mut scheduler = TaskScheduler::new(1).unwrap();
        assert_eq!(
            scheduler.schedule("task:a", "A", vec!["missing".into()], 0, 10),
            Err(SchedulerError::MissingDependency("missing".into()))
        );
        scheduler.schedule("task:a", "A", vec![], 0, 10).unwrap();
        assert_eq!(
            scheduler.schedule("task:a", "duplicate", vec![], 0, 10),
            Err(SchedulerError::DuplicateTask("task:a".into()))
        );
    }

    #[test]
    fn failed_dependency_cannot_be_scheduled_as_prerequisite() {
        let mut scheduler = TaskScheduler::new(1).unwrap();
        scheduler.schedule("task:a", "A", vec![], 0, 10).unwrap();
        scheduler.dispatch_ready(0);
        scheduler.fail("task:a").unwrap();
        assert_eq!(
            scheduler.schedule("task:b", "B", vec!["task:a".into()], 1, 10),
            Err(SchedulerError::DependencyFailed("task:a".into()))
        );
    }

    #[test]
    fn already_queued_dependents_fail_if_prerequisite_fails() {
        let mut scheduler = TaskScheduler::new(1).unwrap();
        scheduler.schedule("task:a", "A", vec![], 0, 10).unwrap();
        scheduler.schedule("task:b", "B", vec!["task:a".into()], 0, 10).unwrap();
        scheduler.dispatch_ready(0);
        scheduler.fail("task:a").unwrap();
        assert!(scheduler.dispatch_ready(1).is_empty());
        assert_eq!(scheduler.get("task:b").unwrap().state, TaskState::Failed);
    }
}
