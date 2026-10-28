mod api;
mod assets;
mod credentials;
mod desktop;
mod input;
mod logging;
mod models;
mod queue;
mod results;
mod store;

use api::ApiClient;
use credentials::CredentialStore;
use models::{
    AppSettings, NewTask, ServiceProfile, StoredResult, Task, TaskKind, TaskSource, Usage,
};
use queue::QueueManager;
use serde::Serialize;
use std::path::PathBuf;
use store::Store;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

pub struct AppState {
    pub store: Store,
    pub credentials: CredentialStore,
    pub api: ApiClient,
    pub queue: QueueManager,
    pub portable: bool,
    pub data_dir: PathBuf,
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl AppState {
    fn new(app: &AppHandle) -> Result<Self, String> {
        let portable = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("portable.flag")))
            .is_some_and(|path| path.exists());
        let data_dir = if portable {
            std::env::current_exe()
                .map_err(|error| error.to_string())?
                .parent()
                .map(|parent| parent.join("data"))
                .ok_or_else(|| "Unable to determine the portable data directory".to_string())?
        } else {
            app.path()
                .app_data_dir()
                .map_err(|error| error.to_string())?
        };
        let log_guard = logging::initialize(&data_dir)?;
        let output_root = data_dir.join("outputs");
        let store = Store::new(data_dir.clone(), output_root.to_string_lossy().into_owned())
            .map_err(|error| error.to_string())?;
        if let Ok(settings) = store.settings() {
            let _ = store.clear_history(Some(settings.history_days));
        }
        let credentials = CredentialStore::new(portable);
        let api = ApiClient::new();
        let queue = QueueManager::new(store.clone(), credentials.clone(), api.clone(), app.clone());
        Ok(Self {
            store,
            credentials,
            api,
            queue,
            portable,
            data_dir,
            _log_guard: log_guard,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    version: String,
    portable: bool,
    settings: AppSettings,
    services: Vec<ServiceProfile>,
    tasks: Vec<Task>,
}

#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
fn get_bootstrap(state: State<'_, AppState>) -> Result<Bootstrap, String> {
    Ok(Bootstrap {
        version: env!("CARGO_PKG_VERSION").into(),
        portable: state.portable,
        settings: state.store.settings().map_err(|error| error.to_string())?,
        services: state
            .store
            .list_services()
            .map_err(|error| error.to_string())?,
        tasks: state
            .store
            .list_tasks()
            .map_err(|error| error.to_string())?,
    })
}

#[tauri::command]
fn save_service(
    mut profile: ServiceProfile,
    api_key: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_base_url(&profile.base_url)?;
    profile.base_url = profile.base_url.trim_end_matches('/').to_string();
    if let Some(key) = api_key.filter(|value| !value.trim().is_empty()) {
        let key = credentials::normalize_api_key(&key);
        if !key.starts_with("sk-") {
            return Err("API Key must use the sk- format".into());
        }
        state
            .credentials
            .set(&profile.id, &key)
            .map_err(|error| error.to_string())?;
    }
    state
        .store
        .save_service(&profile)
        .map_err(|error| error.to_string())?;
    let mut settings = state.store.settings().map_err(|error| error.to_string())?;
    settings.active_service_id = Some(profile.id);
    state
        .store
        .save_settings(&settings)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_service(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .store
        .delete_service(&id)
        .map_err(|error| error.to_string())?;
    let _ = state.credentials.delete(&id);
    Ok(())
}

#[tauri::command]
fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    if settings.concurrency == 0 || settings.concurrency > 32 {
        return Err("Concurrency must be between 1 and 32".into());
    }
    if settings.default_output_root.trim().is_empty() {
        return Err("The default output directory is required".into());
    }
    state
        .store
        .save_settings(&settings)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn validate_service(id: String, state: State<'_, AppState>) -> Result<Usage, String> {
    let profile = state
        .store
        .get_service(&id)
        .map_err(|error| error.to_string())?;
    let key = state
        .credentials
        .get(&id)
        .map_err(|error| error.to_string())?;
    state
        .api
        .usage(&profile, &key)
        .await
        .map_err(|error| error.message)
}

#[tauri::command]
fn enqueue_tasks(new_tasks: Vec<NewTask>, state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    if new_tasks.is_empty() {
        return Err("No tasks were provided".into());
    }
    if new_tasks.len() > 1000 {
        return Err("A single batch cannot contain more than 1000 tasks".into());
    }
    let settings = state.store.settings().map_err(|error| error.to_string())?;
    let batch_id = Uuid::new_v4().to_string();
    let mut priority = state
        .store
        .next_priority()
        .map_err(|error| error.to_string())?;
    let mut tasks = Vec::with_capacity(new_tasks.len());
    for mut new_task in new_tasks {
        state
            .store
            .get_service(&new_task.service_id)
            .map_err(|error| error.to_string())?;
        if new_task.output_root.trim().is_empty() {
            new_task.output_root = settings.default_output_root.clone();
        }
        validate_new_task(&new_task)?;
        let task = new_task.into_task(batch_id.clone(), priority);
        priority += 1;
        tasks.push(task);
    }
    state
        .store
        .create_tasks(&tasks)
        .map_err(|error| error.to_string())?;
    state.queue.wake();
    Ok(tasks)
}

#[tauri::command]
fn list_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    state.store.list_tasks().map_err(|error| error.to_string())
}

#[tauri::command]
fn queue_action(
    id: String,
    action: String,
    confirmed: Option<bool>,
    priority: Option<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    match action.as_str() {
        "pause" => state.queue.pause_task(&id, true),
        "resume" => state.queue.resume(&id),
        "stop" => state.queue.stop(&id),
        "retry" => state.queue.retry(&id, confirmed.unwrap_or(false)),
        "remove" => state.queue.remove(&id),
        "reorder" => state.queue.reorder(&id, priority.unwrap_or(0)),
        _ => return Err("Unknown queue action".into()),
    }
    Ok(())
}

#[tauri::command]
fn pause_queue(paused: bool, state: State<'_, AppState>) {
    state.queue.pause_all(paused);
}

#[tauri::command]
fn load_task_result(id: String, state: State<'_, AppState>) -> Result<StoredResult, String> {
    let task = state
        .store
        .get_task(&id)
        .map_err(|error| error.to_string())?;
    let path = task
        .result_path
        .ok_or_else(|| "The task has no stored result".to_string())?;
    state
        .store
        .load_result(&path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn prepare_task_assets(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let task = state
        .store
        .get_task(&id)
        .map_err(|error| error.to_string())?;
    assets::prepare(&app, &task)
}

#[tauri::command]
fn resolve_result_asset(
    id: String,
    source: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let task = state
        .store
        .get_task(&id)
        .map_err(|error| error.to_string())?;
    assets::resolve(&app, &task, &source)
}

#[tauri::command]
fn save_markdown_copy(
    id: String,
    content: String,
    destination: Option<String>,
    state: State<'_, AppState>,
) -> Result<models::OutputFile, String> {
    let mut task = state
        .store
        .get_task(&id)
        .map_err(|error| error.to_string())?;
    let output =
        results::save_markdown_copy(&task, &content, destination).map_err(|error| error.message)?;
    task.outputs.push(output.clone());
    state
        .store
        .save_task(&task)
        .map_err(|error| error.to_string())?;
    Ok(output)
}

#[tauri::command]
fn export_task_result(
    id: String,
    format: String,
    state: State<'_, AppState>,
) -> Result<models::OutputFile, String> {
    let mut task = state
        .store
        .get_task(&id)
        .map_err(|error| error.to_string())?;
    let result_path = task
        .result_path
        .clone()
        .ok_or_else(|| "The task has no stored result".to_string())?;
    let stored = state
        .store
        .load_result(&result_path)
        .map_err(|error| error.to_string())?;
    let parse = stored
        .parse
        .ok_or_else(|| "Only parse results can be exported as HTML or text".to_string())?;
    let output = results::export_parse(&task, &parse, &format).map_err(|error| error.message)?;
    task.outputs.push(output.clone());
    state
        .store
        .save_task(&task)
        .map_err(|error| error.to_string())?;
    Ok(output)
}

#[tauri::command]
fn clear_history(
    older_than_days: Option<u32>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    state
        .store
        .clear_history(older_than_days)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn export_diagnostics(destination: String, state: State<'_, AppState>) -> Result<PathBuf, String> {
    logging::export_diagnostics(
        &state.data_dir,
        &destination,
        &state.store.settings().map_err(|error| error.to_string())?,
        &state
            .store
            .list_services()
            .map_err(|error| error.to_string())?,
        &state
            .store
            .list_tasks()
            .map_err(|error| error.to_string())?,
    )
}

#[tauri::command]
fn preflight_files(
    paths: Vec<String>,
    recursive: bool,
    service_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<models::PreflightResult>, String> {
    let profile = state
        .store
        .get_service(&service_id)
        .map_err(|error| error.to_string())?;
    Ok(input::preflight_paths(&paths, recursive, &profile))
}

#[tauri::command]
fn import_clipboard_image(
    service_id: String,
    state: State<'_, AppState>,
) -> Result<models::PreflightResult, String> {
    let profile = state
        .store
        .get_service(&service_id)
        .map_err(|error| error.to_string())?;
    input::import_clipboard(state.store.data_dir(), &profile)
}

#[tauri::command]
fn supported_extensions() -> Vec<&'static str> {
    input::SUPPORTED_EXTENSIONS.to_vec()
}

fn validate_base_url(value: &str) -> Result<(), String> {
    let url = url::Url::parse(value).map_err(|_| "The service Base URL is invalid")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Only HTTP(S) service addresses are supported".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Base URL cannot contain credentials, query parameters, or fragments".into());
    }
    Ok(())
}

fn validate_new_task(task: &NewTask) -> Result<(), String> {
    if let TaskSource::Url { url } = &task.source {
        let parsed = url::Url::parse(url).map_err(|_| "The source file URL is invalid")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("Only HTTP(S) source URLs are supported".into());
        }
    }
    match task.kind {
        TaskKind::Parse => {
            if task.formats.is_empty()
                || task
                    .formats
                    .iter()
                    .any(|format| !matches!(format.as_str(), "md" | "markdown" | "json" | "zip"))
            {
                return Err("The requested output formats are invalid".into());
            }
            let has_zip = task.formats.iter().any(|format| format == "zip");
            let image_is_file = task.parse_options.element_formats.image == "file";
            if has_zip != image_is_file {
                return Err("ZIP output and image=file must be selected together".into());
            }
        }
        TaskKind::Office => {
            let is_pdf = match &task.source {
                TaskSource::Local { path } | TaskSource::Clipboard { path } => {
                    std::path::Path::new(path)
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| value.eq_ignore_ascii_case("pdf"))
                }
                TaskSource::Url { url } => url::Url::parse(url)
                    .map(|value| value.path().to_lowercase().ends_with(".pdf"))
                    .unwrap_or(false),
            };
            if !is_pdf {
                return Err("Office conversion only supports PDF files".into());
            }
            if task.office_options.target_type == "ppt"
                && task.office_options.layout_mode != "layout"
            {
                return Err("PowerPoint conversion only supports layout mode".into());
            }
            if !matches!(task.office_options.target_type.as_str(), "word" | "ppt") {
                return Err("The Office target type is invalid".into());
            }
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let state = AppState::new(&app.handle())?;
            app.manage(state);
            desktop::configure(&app.handle())?;
            tracing::info!("SoMarkClient CE initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            get_bootstrap,
            save_service,
            delete_service,
            save_settings,
            validate_service,
            enqueue_tasks,
            list_tasks,
            queue_action,
            pause_queue,
            load_task_result,
            prepare_task_assets,
            resolve_result_asset,
            save_markdown_copy,
            export_task_result,
            clear_history,
            export_diagnostics,
            preflight_files,
            import_clipboard_image,
            supported_extensions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SoMarkClient CE");
}
