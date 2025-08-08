//! # DAG Execution Module
//!
//! This module provides Directed Acyclic Graph (DAG) execution support for the processing pipeline.
//! It enables complex task orchestration with dependency management, retry logic, and SLA monitoring.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::config::model::{
    BackoffStrategy, DagDefinition, DagsConfig, RetryConfig, TaskDefinition,
};
use crate::error::{ConfigError, ProcessingError, Result};
use crate::processing::traits::{PipelineStage, ProcessingContext};

/// DAG execution engine that manages task dependencies and execution
pub struct DagExecutor {
    /// DAG configuration
    config: DagsConfig,
    /// Task registry mapping task names to pipeline stages
    tasks: HashMap<String, Box<dyn PipelineStage>>,
    /// Execution statistics
    stats: DagExecutionStats,
}

/// Statistics for DAG execution
#[derive(Debug, Clone, Default)]
pub struct DagExecutionStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub retried_tasks: usize,
    pub total_execution_time: Duration,
    pub task_execution_times: HashMap<String, Duration>,
}

/// Task execution state
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Retrying,
    Skipped,
}

/// Task execution result
#[derive(Debug)]
pub struct TaskExecutionResult {
    pub task_name: String,
    pub state: TaskState,
    pub execution_time: Duration,
    pub retry_count: u32,
    pub error: Option<String>,
}

/// DAG execution context
pub struct DagExecutionContext {
    pub processing_context: ProcessingContext,
    pub task_states: HashMap<String, TaskState>,
    pub task_results: HashMap<String, TaskExecutionResult>,
    pub execution_start: Instant,
}

impl DagExecutor {
    /// Create a new DAG executor with the given configuration
    pub fn new(config: DagsConfig) -> Self {
        Self {
            config,
            tasks: HashMap::new(),
            stats: DagExecutionStats::default(),
        }
    }

    /// Register a task with the DAG executor
    pub fn register_task(&mut self, name: String, stage: Box<dyn PipelineStage>) -> Result<()> {
        if self.tasks.contains_key(&name) {
            return Err(ProcessingError::PipelineError {
                stage: name,
                message: "Task already registered".to_string(),
            }
            .into());
        }

        self.tasks.insert(name, stage);
        Ok(())
    }

    /// Validate the DAG configuration and task dependencies
    pub fn validate(&self) -> Result<()> {
        for dag in self.config.dags.values() {
            self.validate_dag(dag)?;
        }
        Ok(())
    }

    /// Validate a single DAG definition
    fn validate_dag(&self, dag: &DagDefinition) -> Result<()> {
        // Check for task name uniqueness
        let mut task_names = HashSet::new();
        for task in dag.tasks.values() {
            if !task_names.insert(&task.name) {
                return Err(ConfigError::DagValidationError {
                    dag_name: dag.name.clone(),
                    message: format!("Duplicate task name: {}", task.name),
                }
                .into());
            }
        }

        // Check that all dependencies exist
        for task in dag.tasks.values() {
            for dep in &task.depends_on {
                if !task_names.contains(dep) {
                    return Err(ConfigError::DagValidationError {
                        dag_name: dag.name.clone(),
                        message: format!(
                            "Task '{}' depends on non-existent task '{}'",
                            task.name, dep
                        ),
                    }
                    .into());
                }
            }
        }

        // Check for cycles
        self.detect_cycles(dag)?;

        // Validate retry and backoff configurations
        for task in dag.tasks.values() {
            self.validate_retry_config(&task.retry, &dag.name, &task.name)?;
        }

        Ok(())
    }

    /// Detect cycles in the DAG using topological sort
    fn detect_cycles(&self, dag: &DagDefinition) -> Result<()> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize in-degree and graph
        for task in dag.tasks.values() {
            in_degree.insert(task.name.clone(), 0);
            graph.insert(task.name.clone(), Vec::new());
        }

        // Build the graph and calculate in-degrees
        for task in dag.tasks.values() {
            for dep in &task.depends_on {
                graph.get_mut(dep).unwrap().push(task.name.clone());
                *in_degree.get_mut(&task.name).unwrap() += 1;
            }
        }

        // Topological sort using Kahn's algorithm
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut processed = 0;

        // Add all nodes with in-degree 0 to the queue
        for (task, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(task.clone());
            }
        }

        while let Some(task) = queue.pop_front() {
            processed += 1;

            // Process all neighbors
            if let Some(neighbors) = graph.get(&task) {
                for neighbor in neighbors {
                    let degree = in_degree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        // If we didn't process all nodes, there's a cycle
        if processed != dag.tasks.values().len() {
            return Err(ConfigError::DagValidationError {
                dag_name: dag.name.clone(),
                message: "Cycle detected in task dependencies".to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Validate retry configuration
    fn validate_retry_config(
        &self,
        retry: &RetryConfig,
        dag_name: &str,
        task_name: &str,
    ) -> Result<()> {
        if retry.max_attempts == 0 {
            return Err(ConfigError::DagValidationError {
                dag_name: dag_name.to_string(),
                message: format!("Task '{task_name}' has invalid retry max_attempts: 0"),
            }
            .into());
        }

        if let Some(delay) = retry.initial_delay {
            if delay.as_secs() == 0 && delay.subsec_millis() == 0 {
                return Err(ConfigError::DagValidationError {
                    dag_name: dag_name.to_string(),
                    message: format!("Task '{task_name}' has invalid retry initial_delay: 0"),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Execute a DAG by name
    pub async fn execute_dag(
        &mut self,
        dag_name: &str,
        context: ProcessingContext,
    ) -> Result<DagExecutionContext> {
        let dag = self
            .config
            .dags
            .iter()
            .find(|(_, d)| d.name == dag_name)
            .map(|(_, dag)| dag)
            .ok_or_else(|| ProcessingError::PipelineError {
                stage: dag_name.to_string(),
                message: "DAG not found".to_string(),
            })?;

        info!("Starting DAG execution: {}", dag_name);
        let execution_start = Instant::now();

        let mut dag_context = DagExecutionContext {
            processing_context: context,
            task_states: HashMap::new(),
            task_results: HashMap::new(),
            execution_start,
        };

        // Execute tasks in dependency order
        let dag_copy = dag.clone();

        // Initialize task states
        for task in dag_copy.tasks.values() {
            dag_context
                .task_states
                .insert(task.name.clone(), TaskState::Pending);
        }

        self.execute_tasks(&dag_copy, &mut dag_context).await?;

        // Update statistics
        self.stats.total_execution_time = execution_start.elapsed();
        self.stats.total_tasks = dag_copy.tasks.values().len();
        self.stats.completed_tasks = dag_context
            .task_states
            .values()
            .filter(|&state| *state == TaskState::Completed)
            .count();
        self.stats.failed_tasks = dag_context
            .task_states
            .values()
            .filter(|&state| *state == TaskState::Failed)
            .count();

        info!(
            "DAG execution completed: {} ({}ms)",
            dag_name,
            self.stats.total_execution_time.as_millis()
        );

        Ok(dag_context)
    }

    /// Execute tasks in the DAG
    async fn execute_tasks(
        &mut self,
        dag: &DagDefinition,
        context: &mut DagExecutionContext,
    ) -> Result<()> {
        let mut remaining_tasks: HashSet<String> =
            dag.tasks.values().map(|t| t.name.clone()).collect();

        while !remaining_tasks.is_empty() {
            let ready_tasks = self.find_ready_tasks(dag, context, &remaining_tasks);

            if ready_tasks.is_empty() {
                // Check if we have any failed tasks that are blocking progress
                let failed_tasks: Vec<_> = context
                    .task_states
                    .iter()
                    .filter(|(_, state)| **state == TaskState::Failed)
                    .map(|(name, _)| name.clone())
                    .collect();

                if !failed_tasks.is_empty() {
                    return Err(ProcessingError::PipelineError {
                        stage: "DAG".to_string(),
                        message: format!(
                            "DAG execution blocked by failed tasks: {failed_tasks:?}"
                        ),
                    }
                    .into());
                }

                return Err(ProcessingError::PipelineError {
                    stage: "DAG".to_string(),
                    message: "No ready tasks found, possible deadlock".to_string(),
                }
                .into());
            }

            // Execute ready tasks (could be parallelized in the future)
            for task_name in ready_tasks {
                let task_def = dag
                    .tasks
                    .iter()
                    .find(|(_, t)| t.name == task_name)
                    .map(|(_, t)| t)
                    .unwrap();
                self.execute_task(task_def, context).await?;
                remaining_tasks.remove(&task_name);
            }
        }

        Ok(())
    }

    /// Find tasks that are ready to execute (all dependencies completed)
    fn find_ready_tasks(
        &self,
        dag: &DagDefinition,
        context: &DagExecutionContext,
        remaining: &HashSet<String>,
    ) -> Vec<String> {
        let mut ready = Vec::new();

        for task in dag.tasks.values() {
            if !remaining.contains(&task.name) {
                continue;
            }

            if context.task_states.get(&task.name) != Some(&TaskState::Pending) {
                continue;
            }

            // Check if all dependencies are completed
            let all_deps_completed = task
                .depends_on
                .iter()
                .all(|dep| context.task_states.get(dep) == Some(&TaskState::Completed));

            if all_deps_completed {
                ready.push(task.name.clone());
            }
        }

        ready
    }

    /// Execute a single task with retry logic
    async fn execute_task(
        &mut self,
        task: &TaskDefinition,
        context: &mut DagExecutionContext,
    ) -> Result<()> {
        let task_name = &task.name;
        info!("Executing task: {}", task_name);

        context
            .task_states
            .insert(task_name.clone(), TaskState::Running);
        let task_start = Instant::now();

        let mut retry_count = 0;
        let max_attempts = task.retry.max_attempts.max(1);
        let retry_config = task.retry.clone();

        loop {
            // Execute the stage
            let execution_result = {
                let stage = self.tasks.get_mut(task_name).ok_or_else(|| {
                    ProcessingError::PipelineError {
                        stage: task_name.clone(),
                        message: "Task implementation not found".to_string(),
                    }
                })?;

                stage.execute(&mut context.processing_context).await
            };

            match execution_result {
                Ok(_) => {
                    let execution_time = task_start.elapsed();
                    info!(
                        "Task completed: {} ({}ms)",
                        task_name,
                        execution_time.as_millis()
                    );

                    context
                        .task_states
                        .insert(task_name.clone(), TaskState::Completed);
                    context.task_results.insert(
                        task_name.clone(),
                        TaskExecutionResult {
                            task_name: task_name.clone(),
                            state: TaskState::Completed,
                            execution_time,
                            retry_count,
                            error: None,
                        },
                    );

                    self.stats
                        .task_execution_times
                        .insert(task_name.clone(), execution_time);
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    warn!(
                        "Task failed: {} (attempt {}/{}): {}",
                        task_name, retry_count, max_attempts, e
                    );

                    if retry_count >= max_attempts {
                        let execution_time = task_start.elapsed();
                        error!(
                            "Task failed permanently: {} after {} attempts",
                            task_name, retry_count
                        );

                        context
                            .task_states
                            .insert(task_name.clone(), TaskState::Failed);
                        context.task_results.insert(
                            task_name.clone(),
                            TaskExecutionResult {
                                task_name: task_name.clone(),
                                state: TaskState::Failed,
                                execution_time,
                                retry_count,
                                error: Some(e.to_string()),
                            },
                        );

                        self.stats.failed_tasks += 1;
                        return Err(e);
                    }

                    // Calculate delay before any mutable borrows
                    let delay = self.calculate_backoff_delay(&retry_config, retry_count);

                    // Update retry stats
                    self.stats.retried_tasks += 1;

                    // Apply backoff strategy
                    context
                        .task_states
                        .insert(task_name.clone(), TaskState::Retrying);
                    debug!("Retrying task {} in {}ms", task_name, delay.as_millis());
                    sleep(delay).await;
                }
            }
        }

        Ok(())
    }

    /// Calculate backoff delay based on strategy and attempt number
    fn calculate_backoff_delay(&self, retry_config: &RetryConfig, attempt: u32) -> Duration {
        match retry_config.backoff {
            BackoffStrategy::Fixed => retry_config
                .initial_delay
                .unwrap_or(Duration::from_millis(retry_config.delay)),
            BackoffStrategy::Exponential => {
                let multiplier = 2_u64.pow(attempt - 1);
                let base_delay = retry_config
                    .initial_delay
                    .unwrap_or(Duration::from_millis(retry_config.delay));
                Duration::from_millis(base_delay.as_millis() as u64 * multiplier)
            }
            BackoffStrategy::Linear => {
                let base_delay = retry_config
                    .initial_delay
                    .unwrap_or(Duration::from_millis(retry_config.delay));
                Duration::from_millis(base_delay.as_millis() as u64 * attempt as u64)
            }
        }
    }

    /// Get execution statistics
    pub fn stats(&self) -> &DagExecutionStats {
        &self.stats
    }

    /// Reset execution statistics
    pub fn reset_stats(&mut self) {
        self.stats = DagExecutionStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::TaskDefaults;
    use std::time::Duration;

    fn create_test_dag() -> DagDefinition {
        let mut tasks = HashMap::new();
        tasks.insert("task1".to_string(), TaskDefinition {
            name: "task1".to_string(),
            task_type: "loader".to_string(),
            depends_on: vec![],
            retry: RetryConfig {
                max_attempts: 3,
                backoff: BackoffStrategy::Exponential,
                delay: 100,
                initial_delay: Some(Duration::from_millis(100)),
            },
            sla: None,
            parameters: HashMap::new(),
            description: Some("First task".to_string()),
            stage: "loader".to_string(),
            timeout: Some(Duration::from_secs(30)),
        });
        tasks.insert("task2".to_string(), TaskDefinition {
            name: "task2".to_string(),
            task_type: "transformer".to_string(),
            depends_on: vec!["task1".to_string()],
            retry: RetryConfig::default(),
            sla: None,
            parameters: HashMap::new(),
            description: Some("Second task".to_string()),
            stage: "transformer".to_string(),
            timeout: None,
        });
        
        DagDefinition {
            name: "test_dag".to_string(),
            description: Some("Test DAG".to_string()),
            tasks,
            defaults: TaskDefaults {
                retry: Some(RetryConfig {
                    max_attempts: 1,
                    backoff: BackoffStrategy::Fixed,
                    delay: 50,
                    initial_delay: Some(Duration::from_millis(50)),
                }),
                sla: None,
                timeout: Some(60),
            },
        }
    }

    #[test]
    fn test_dag_validation_success() {
        let mut dags = HashMap::new();
        dags.insert("test_dag".to_string(), create_test_dag());
        let config = DagsConfig {
            config_version: 1,
            dags,
        };
        let executor = DagExecutor::new(config);
        assert!(executor.validate().is_ok());
    }

    #[test]
    fn test_dag_validation_cycle_detection() {
        let mut dag = create_test_dag();
        // Create a cycle: task1 -> task2 -> task1
        dag.tasks.get_mut("task1").unwrap().depends_on = vec!["task2".to_string()];

        let mut dags = HashMap::new();
        dags.insert("test_dag".to_string(), dag);
        let config = DagsConfig {
            config_version: 1,
            dags,
        };
        let executor = DagExecutor::new(config);
        assert!(executor.validate().is_err());
    }

    #[test]
    fn test_dag_validation_missing_dependency() {
        let mut dag = create_test_dag();
        dag.tasks.get_mut("task2").unwrap().depends_on = vec!["nonexistent_task".to_string()];

        let mut dags = HashMap::new();
        dags.insert("test_dag".to_string(), dag);
        let config = DagsConfig {
            config_version: 1,
            dags,
        };
        let executor = DagExecutor::new(config);
        assert!(executor.validate().is_err());
    }

    #[test]
    fn test_backoff_calculation() {
        let executor = DagExecutor::new(DagsConfig {
            config_version: 1,
            dags: HashMap::new(),
        });
        let retry_config = RetryConfig {
            max_attempts: 3,
            initial_delay: Some(Duration::from_millis(100)),
            backoff: BackoffStrategy::Exponential,
            delay: 100,
        };

        assert_eq!(
            executor.calculate_backoff_delay(&retry_config, 1),
            Duration::from_millis(100)
        );
        assert_eq!(
            executor.calculate_backoff_delay(&retry_config, 2),
            Duration::from_millis(200)
        );
        assert_eq!(
            executor.calculate_backoff_delay(&retry_config, 3),
            Duration::from_millis(400)
        );
    }

    #[test]
    fn test_find_ready_tasks() {
        let dag = create_test_dag();
        let executor = DagExecutor::new(DagsConfig {
            config_version: 1,
            dags: HashMap::new(),
        });

        let mut context = DagExecutionContext {
            processing_context: ProcessingContext::new(Default::default()),
            task_states: HashMap::new(),
            task_results: HashMap::new(),
            execution_start: Instant::now(),
        };

        // Initially, only task1 should be ready (no dependencies)
        context
            .task_states
            .insert("task1".to_string(), TaskState::Pending);
        context
            .task_states
            .insert("task2".to_string(), TaskState::Pending);

        let remaining: HashSet<String> = vec!["task1".to_string(), "task2".to_string()]
            .into_iter()
            .collect();
        let ready = executor.find_ready_tasks(&dag, &context, &remaining);
        assert_eq!(ready, vec!["task1"]);

        // After task1 completes, task2 should be ready
        context
            .task_states
            .insert("task1".to_string(), TaskState::Completed);
        let remaining: HashSet<String> = vec!["task2".to_string()].into_iter().collect();
        let ready = executor.find_ready_tasks(&dag, &context, &remaining);
        assert_eq!(ready, vec!["task2"]);
    }
}
