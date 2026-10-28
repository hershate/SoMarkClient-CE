use crate::api::{ApiClient, ProgressCallback};
use crate::credentials::CredentialStore;
use crate::models::{
    AppError, OfficeData, ServiceProfile, StoredResult, Task, TaskKind, TaskMode, TaskStatus,
};
use crate::results::{persist_office, persist_parse};
use crate::store::Store;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

#[derive(Debug)]
enum QueueCommand {
    Wake,
    PauseAll(bool),
    PauseTask(String, bool),
    Stop(String),
    Resume(String),
    Retry(String, bool),
    Remove(String),
    Reorder(String, i64),
}

#[derive(Clone)]
pub struct QueueManager {
    sender: mpsc::UnboundedSender<QueueCommand>,
}

impl QueueManager {
    pub fn new(store: Store, credentials: CredentialStore, api: ApiClient, app: AppHandle) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        tauri::async_runtime::spawn(run_actor(store, credentials, api, app, receiver));
        Self { sender }
    }

    pub fn wake(&self) {
        let _ = self.sender.send(QueueCommand::Wake);
    }

    pub fn pause_all(&self, paused: bool) {
        let _ = self.sender.send(QueueCommand::PauseAll(paused));
    }

    pub fn pause_task(&self, id: &str, paused: bool) {
        let _ = self.sender.send(QueueCommand::PauseTask(id.into(), paused));
    }

    pub fn stop(&self, id: &str) {
        let _ = self.sender.send(QueueCommand::Stop(id.into()));
    }

    pub fn resume(&self, id: &str) {
        let _ = self.sender.send(QueueCommand::Resume(id.into()));
    }

    pub fn retry(&self, id: &str, confirmed: bool) {
        let _ = self.sender.send(QueueCommand::Retry(id.into(), confirmed));
    }

    pub fn remove(&self, id: &str) {
        let _ = self.sender.send(QueueCommand::Remove(id.into()));
    }

    pub fn reorder(&self, id: &str, priority: i64) {
        let _ = self.sender.send(QueueCommand::Reorder(id.into(), priority));
    }
}

async fn run_actor(
    store: Store,
    credentials: CredentialStore,
    api: ApiClient,
    app: AppHandle,
    mut receiver: mpsc::UnboundedReceiver<QueueCommand>,
) {
    let mut workers = JoinSet::new();
    let mut running = HashMap::new();
    let mut paused = false;
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            command = receiver.recv() => {
                let Some(command) = command else { break };
                handle_command(command, &store, &app, &running, &mut paused);
            }
            completed = workers.join_next(), if !workers.is_empty() => {
                if let Some(Ok(id)) = completed {
                    running.remove(&id);
                }
            }
            _ = interval.tick() => {
                if paused {
                    continue;
                }
                let global_limit = store.settings().map(|value| value.concurrency.max(1)).unwrap_or(1);
                let Ok(tasks) = store.runnable_tasks() else { continue };
                for task in tasks {
                    if running.len() >= global_limit || running.contains_key(&task.id) {
                        continue;
                    }
                    let Ok(profile) = store.get_service(&task.service_id) else { continue };
                    let profile_limit = if profile.higher_concurrency_approved {
                        profile.max_concurrency.max(1)
                    } else {
                        1
                    };
                    let service_running = running.values().filter(|id| **id == task.service_id).count();
                    if service_running >= profile_limit {
                        continue;
                    }
                    running.insert(task.id.clone(), task.service_id.clone());
                    let id = task.id.clone();
                    let worker_store = store.clone();
                    let worker_credentials = credentials.clone();
                    let worker_api = api.clone();
                    let worker_app = app.clone();
                    workers.spawn(async move {
                        process_task(id.clone(), worker_store, worker_credentials, worker_api, worker_app).await;
                        id
                    });
                    if running.len() >= global_limit {
                        break;
                    }
                }
            }
        }
    }
}

fn handle_command(
    command: QueueCommand,
    store: &Store,
    app: &AppHandle,
    running: &HashMap<String, String>,
    paused: &mut bool,
) {
    match command {
        QueueCommand::Wake => {}
        QueueCommand::PauseAll(value) => {
            *paused = value;
            if let Ok(tasks) = store.list_tasks() {
                for mut task in tasks {
                    if value && task.status == TaskStatus::Queued {
                        task.status = TaskStatus::Paused;
                        save_task(store, app, &mut task);
                    } else if !value && task.status == TaskStatus::Paused {
                        task.status = if task.remote_id.is_some() {
                            TaskStatus::Waiting
                        } else {
                            TaskStatus::Queued
                        };
                        save_task(store, app, &mut task);
                    }
                }
            }
        }
        QueueCommand::PauseTask(id, value) => {
            if let Ok(mut task) = store.get_task(&id) {
                if value
                    && matches!(
                        task.status,
                        TaskStatus::Queued | TaskStatus::Waiting | TaskStatus::Downloading
                    )
                {
                    task.status = TaskStatus::Paused;
                    save_task(store, app, &mut task);
                } else if !value && task.status == TaskStatus::Paused {
                    task.status = resumable_status(&task);
                    save_task(store, app, &mut task);
                }
            }
        }
        QueueCommand::Stop(id) => {
            if let Ok(mut task) = store.get_task(&id) {
                if !task.status.is_terminal() {
                    task.status = TaskStatus::Stopped;
                    save_task(store, app, &mut task);
                }
            }
        }
        QueueCommand::Resume(id) => {
            if let Ok(mut task) = store.get_task(&id) {
                if matches!(task.status, TaskStatus::Stopped | TaskStatus::Paused) {
                    task.status = resumable_status(&task);
                    task.error = None;
                    save_task(store, app, &mut task);
                }
            }
        }
        QueueCommand::Retry(id, confirmed) => {
            if let Ok(mut task) = store.get_task(&id) {
                if task.status == TaskStatus::SubmissionUnknown && !confirmed {
                    continue_without_retry(store, app, &mut task);
                    return;
                }
                if task.status == TaskStatus::PartialSuccess {
                    task.status = TaskStatus::Downloading;
                } else if task.remote_status.as_deref().is_some_and(is_failed) {
                    task.remote_id = None;
                    task.remote_status = None;
                    task.status = TaskStatus::Queued;
                } else if task.remote_id.is_some() {
                    task.status = TaskStatus::Waiting;
                } else {
                    task.status = TaskStatus::Queued;
                }
                task.error = None;
                task.retry_count += 1;
                save_task(store, app, &mut task);
            }
        }
        QueueCommand::Remove(id) => {
            if !running.contains_key(&id) {
                let _ = store.delete_task(&id);
                let _ = app.emit("task-removed", id);
            }
        }
        QueueCommand::Reorder(id, priority) => {
            let _ = store.reorder_task(&id, priority);
        }
    }
}

fn continue_without_retry(store: &Store, app: &AppHandle, task: &mut Task) {
    task.error = Some(AppError::new(
        "confirmationRequired",
        "Retrying may create a duplicate cloud task. Confirm the retry explicitly.",
    ));
    save_task(store, app, task);
}

fn resumable_status(task: &Task) -> TaskStatus {
    if task.result_path.is_some() {
        TaskStatus::Downloading
    } else if task.remote_id.is_some() {
        TaskStatus::Waiting
    } else {
        TaskStatus::Queued
    }
}

async fn process_task(
    id: String,
    store: Store,
    credentials: CredentialStore,
    api: ApiClient,
    app: AppHandle,
) {
    let Ok(task) = store.get_task(&id) else {
        return;
    };
    if matches!(task.status, TaskStatus::Stopped | TaskStatus::Paused) {
        return;
    }
    let result = match task.status {
        TaskStatus::Waiting => poll_task(task, store, credentials, api, app).await,
        TaskStatus::Downloading => finish_task(task, store, api, app).await,
        _ => submit_task(task, store, credentials, api, app).await,
    };
    if let Err(error) = result {
        tracing::error!(kind = %error.kind, "task processing failed");
    }
}

async fn submit_task(
    mut task: Task,
    store: Store,
    credentials: CredentialStore,
    api: ApiClient,
    app: AppHandle,
) -> Result<(), AppError> {
    let profile = store
        .get_service(&task.service_id)
        .map_err(|_| AppError::new("configuration", "The service profile no longer exists"))?;
    let key = credentials
        .get(&profile.id)
        .map_err(|_| AppError::new("authentication", "API Key is not configured"))?;
    task.status = TaskStatus::Uploading;
    task.progress = 0.0;
    task.error = None;
    save_task(&store, &app, &mut task);
    let progress = progress_callback(task.id.clone(), app.clone());
    let is_async = task.mode == TaskMode::Async
        || (task.mode == TaskMode::Auto
            && (task.file_size > 30 * 1024 * 1024 || batch_size(&store, &task.batch_id) > 1));

    match task.kind {
        TaskKind::Parse => {
            let response = if is_async {
                api.parse_submit(&profile, &key, &task, progress).await
            } else {
                api.parse_sync(&profile, &key, &task, progress).await
            };
            match response {
                Ok((data, warnings)) => {
                    if interrupted(&store, &task.id) {
                        return Ok(());
                    }
                    task.warnings = warnings;
                    task.progress = 1.0;
                    task.remote_id = data.task_id.clone().or(task.remote_id);
                    task.remote_status = data.status.clone();
                    if !is_async || data.result.is_some() {
                        let result = data.result.ok_or_else(|| {
                            AppError::new("response", "The parse response did not contain a result")
                        })?;
                        task.page_count = data.metadata.and_then(|value| value.page_num);
                        store_result(
                            &store,
                            &mut task,
                            StoredResult {
                                parse: Some(result),
                                office: None,
                            },
                        )?;
                        task.status = TaskStatus::Downloading;
                        save_task(&store, &app, &mut task);
                        finish_task(task, store, api, app).await
                    } else {
                        require_remote_id(&task)?;
                        task.status = TaskStatus::Waiting;
                        save_task(&store, &app, &mut task);
                        poll_task(task, store, credentials, api, app).await
                    }
                }
                Err(error) => {
                    submission_failed(&store, &app, &mut task, error);
                    Ok(())
                }
            }
        }
        TaskKind::Office => {
            let response = if is_async {
                api.office_submit(&profile, &key, &task, progress).await
            } else {
                api.office_sync(&profile, &key, &task, progress).await
            };
            match response {
                Ok((data, _)) => {
                    if interrupted(&store, &task.id) {
                        return Ok(());
                    }
                    task.progress = 1.0;
                    task.remote_id = data.task_id.clone().or(task.remote_id);
                    task.remote_status = data.status.clone();
                    if data.status.as_deref().is_some_and(is_failed) {
                        fail_task(&store, &app, &mut task, office_error(&data));
                        return Ok(());
                    }
                    if data.download_url.is_some() {
                        store_result(
                            &store,
                            &mut task,
                            StoredResult {
                                parse: None,
                                office: Some(data),
                            },
                        )?;
                        task.status = TaskStatus::Downloading;
                        save_task(&store, &app, &mut task);
                        finish_task(task, store, api, app).await
                    } else {
                        require_remote_id(&task)?;
                        task.status = TaskStatus::Waiting;
                        save_task(&store, &app, &mut task);
                        poll_task(task, store, credentials, api, app).await
                    }
                }
                Err(error) => {
                    submission_failed(&store, &app, &mut task, error);
                    Ok(())
                }
            }
        }
    }
}

async fn poll_task(
    mut task: Task,
    store: Store,
    credentials: CredentialStore,
    api: ApiClient,
    app: AppHandle,
) -> Result<(), AppError> {
    let profile = store
        .get_service(&task.service_id)
        .map_err(|_| AppError::new("configuration", "The service profile no longer exists"))?;
    let key = credentials
        .get(&profile.id)
        .map_err(|_| AppError::new("authentication", "API Key is not configured"))?;
    require_remote_id(&task)?;
    loop {
        if interrupted(&store, &task.id) {
            return Ok(());
        }
        task.status = TaskStatus::Waiting;
        save_task(&store, &app, &mut task);
        let remote_id = task.remote_id.clone().unwrap_or_default();
        match task.kind {
            TaskKind::Parse => match api.parse_check(&profile, &key, &remote_id).await {
                Ok((data, warnings)) => {
                    task.warnings = warnings;
                    task.remote_status = data.status.clone();
                    if data.status.as_deref() == Some("success") {
                        let result = data.result.ok_or_else(|| {
                            AppError::new("response", "The completed task did not contain a result")
                        })?;
                        task.page_count = data.metadata.and_then(|value| value.page_num);
                        store_result(
                            &store,
                            &mut task,
                            StoredResult {
                                parse: Some(result),
                                office: None,
                            },
                        )?;
                        task.status = TaskStatus::Downloading;
                        save_task(&store, &app, &mut task);
                        return finish_task(task, store, api, app).await;
                    }
                    if data.status.as_deref().is_some_and(is_failed) {
                        let error = data
                            .error_code
                            .map(|code| {
                                AppError::api(
                                    code,
                                    data.error_message
                                        .unwrap_or_else(|| "Parse task failed".into()),
                                )
                            })
                            .unwrap_or_else(|| AppError::new("task", "Parse task failed"));
                        fail_task(&store, &app, &mut task, error);
                        return Ok(());
                    }
                    save_task(&store, &app, &mut task);
                }
                Err(error) => {
                    if !retry_delay(&store, &app, &mut task, &profile, &error).await {
                        fail_task(&store, &app, &mut task, error);
                        return Ok(());
                    }
                }
            },
            TaskKind::Office => match api.office_check(&profile, &key, &remote_id).await {
                Ok((data, _)) => {
                    task.remote_status = data.status.clone();
                    if data.status.as_deref() == Some("SUCCESS") {
                        store_result(
                            &store,
                            &mut task,
                            StoredResult {
                                parse: None,
                                office: Some(data),
                            },
                        )?;
                        task.status = TaskStatus::Downloading;
                        save_task(&store, &app, &mut task);
                        return finish_task(task, store, api, app).await;
                    }
                    if data.status.as_deref().is_some_and(is_failed) {
                        fail_task(&store, &app, &mut task, office_error(&data));
                        return Ok(());
                    }
                    save_task(&store, &app, &mut task);
                }
                Err(error) => {
                    if !retry_delay(&store, &app, &mut task, &profile, &error).await {
                        fail_task(&store, &app, &mut task, error);
                        return Ok(());
                    }
                }
            },
        }
        tokio::time::sleep(Duration::from_secs(4)).await;
    }
}

async fn finish_task(
    mut task: Task,
    store: Store,
    api: ApiClient,
    app: AppHandle,
) -> Result<(), AppError> {
    let profile = store
        .get_service(&task.service_id)
        .map_err(|_| AppError::new("configuration", "The service profile no longer exists"))?;
    let result_path = task
        .result_path
        .clone()
        .ok_or_else(|| AppError::new("result", "The stored result is unavailable"))?;
    let result = store
        .load_result(&result_path)
        .map_err(|_| AppError::new("result", "The stored result cannot be read"))?;
    loop {
        if interrupted(&store, &task.id) {
            return Ok(());
        }
        task.status = TaskStatus::Downloading;
        save_task(&store, &app, &mut task);
        let progress = progress_callback(task.id.clone(), app.clone());
        let (outputs, failure) = match (
            task.kind.clone(),
            result.parse.as_ref(),
            result.office.as_ref(),
        ) {
            (TaskKind::Parse, Some(parse), _) => {
                persist_parse(&task, parse, &api, &profile, progress).await
            }
            (TaskKind::Office, _, Some(office)) => {
                persist_office(&task, office, &api, &profile, progress).await
            }
            _ => {
                return Err(AppError::new(
                    "result",
                    "The stored result type does not match the task",
                ))
            }
        };
        task.outputs = outputs;
        match failure {
            None => {
                task.status = TaskStatus::Success;
                task.error = None;
                task.progress = 1.0;
                save_task(&store, &app, &mut task);
                notify_completion(&store, &app, &task);
                return Ok(());
            }
            Some(error) if error.retryable && can_retry(&store, &task, &profile) => {
                task.error = Some(error);
                task.retry_count += 1;
                save_task(&store, &app, &mut task);
                tokio::time::sleep(backoff(task.retry_count)).await;
            }
            Some(error) => {
                task.status = TaskStatus::PartialSuccess;
                task.error = Some(error);
                save_task(&store, &app, &mut task);
                notify_completion(&store, &app, &task);
                return Ok(());
            }
        }
    }
}

fn store_result(store: &Store, task: &mut Task, result: StoredResult) -> Result<(), AppError> {
    task.result_path = Some(
        store
            .save_result(&task.id, &result)
            .map_err(|_| AppError::new("result", "Unable to persist the API result"))?,
    );
    Ok(())
}

fn submission_failed(store: &Store, app: &AppHandle, task: &mut Task, error: AppError) {
    if error.retryable {
        task.status = TaskStatus::SubmissionUnknown;
    } else {
        task.status = TaskStatus::Failed;
        notify_completion(store, app, task);
    }
    task.error = Some(error);
    save_task(store, app, task);
}

fn fail_task(store: &Store, app: &AppHandle, task: &mut Task, error: AppError) {
    task.status = TaskStatus::Failed;
    task.error = Some(error);
    save_task(store, app, task);
    notify_completion(store, app, task);
}

async fn retry_delay(
    store: &Store,
    app: &AppHandle,
    task: &mut Task,
    profile: &ServiceProfile,
    error: &AppError,
) -> bool {
    if !error.retryable || !can_retry(store, task, profile) {
        return false;
    }
    task.error = Some(error.clone());
    task.retry_count += 1;
    save_task(store, app, task);
    tokio::time::sleep(backoff(task.retry_count)).await;
    true
}

fn can_retry(store: &Store, task: &Task, profile: &ServiceProfile) -> bool {
    let settings_limit = store.settings().map(|value| value.retry_limit).unwrap_or(3);
    task.retry_count < settings_limit.min(profile.max_retries)
}

fn backoff(retry_count: u32) -> Duration {
    Duration::from_secs(2_u64.saturating_pow(retry_count.min(5)).min(30))
}

fn save_task(store: &Store, app: &AppHandle, task: &mut Task) {
    task.updated_at = chrono::Utc::now().to_rfc3339();
    if let Err(error) = store.save_task(task) {
        tracing::error!(%error, "unable to save task");
        return;
    }
    let _ = app.emit("task-updated", &*task);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let active = store
            .list_tasks()
            .map(|tasks| {
                tasks
                    .iter()
                    .filter(|item| {
                        matches!(
                            item.status,
                            TaskStatus::Uploading | TaskStatus::Waiting | TaskStatus::Downloading
                        )
                    })
                    .count()
            })
            .unwrap_or(0);
        let _ = tray.set_tooltip(Some(format!("SoMarkClient CE · {active} active")));
    }
}

fn notify_completion(store: &Store, app: &AppHandle, task: &Task) {
    let enabled = store
        .settings()
        .map(|value| value.notifications)
        .unwrap_or(true);
    if !enabled {
        return;
    }
    let title = match task.status {
        TaskStatus::Success => "Task completed",
        TaskStatus::PartialSuccess => "Task partially completed",
        _ => "Task failed",
    };
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(&task.name)
        .show();
}

fn interrupted(store: &Store, id: &str) -> bool {
    store
        .get_task(id)
        .map(|task| matches!(task.status, TaskStatus::Stopped | TaskStatus::Paused))
        .unwrap_or(true)
}

fn require_remote_id(task: &Task) -> Result<(), AppError> {
    if task.remote_id.is_none() {
        Err(AppError::new(
            "response",
            "The API response did not contain a task ID",
        ))
    } else {
        Ok(())
    }
}

fn office_error(data: &OfficeData) -> AppError {
    data.error_code
        .map(|code| {
            AppError::api(
                code,
                data.error_message
                    .clone()
                    .unwrap_or_else(|| "Office conversion failed".into()),
            )
        })
        .unwrap_or_else(|| AppError::new("task", "Office conversion failed"))
}

fn is_failed(status: &str) -> bool {
    status.eq_ignore_ascii_case("failed")
}

fn batch_size(store: &Store, batch_id: &str) -> usize {
    store
        .list_tasks()
        .map(|tasks| {
            tasks
                .iter()
                .filter(|task| task.batch_id == batch_id)
                .count()
        })
        .unwrap_or(1)
}

fn progress_callback(id: String, app: AppHandle) -> ProgressCallback {
    Arc::new(move |sent, total| {
        let _ = app.emit(
            "upload-progress",
            ProgressEvent {
                task_id: id.clone(),
                sent,
                total,
                progress: if total == 0 {
                    0.0
                } else {
                    sent as f64 / total as f64
                },
            },
        );
    })
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    task_id: String,
    sent: u64,
    total: u64,
    progress: f64,
}
