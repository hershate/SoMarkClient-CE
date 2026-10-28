use crate::models::{AppSettings, ServiceProfile, Task};
use crate::results::unique_path;
use serde_json::json;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;

pub fn initialize(data_dir: &Path) -> Result<WorkerGuard, String> {
    let directory = data_dir.join("logs");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let appender = tracing_appender::rolling::daily(directory, "somarkclient.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = tracing_subscriber::EnvFilter::new("somarkclient_ce=info");
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .try_init()
        .map_err(|error| error.to_string())?;
    Ok(guard)
}

pub fn export_diagnostics(
    data_dir: &Path,
    destination: &str,
    settings: &AppSettings,
    services: &[ServiceProfile],
    tasks: &[Task],
) -> Result<PathBuf, String> {
    cleanup_logs(data_dir, settings.log_days);
    let destination = unique_path(PathBuf::from(destination), false);
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file = std::fs::File::create(&destination).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let task_summaries: Vec<_> = tasks
        .iter()
        .map(|task| {
            json!({
                "id": task.id,
                "kind": task.kind,
                "status": task.status,
                "remoteStatus": task.remote_status,
                "retryCount": task.retry_count,
                "errorKind": task.error.as_ref().map(|error| &error.kind),
                "errorCode": task.error.as_ref().and_then(|error| error.code),
                "createdAt": task.created_at,
                "updatedAt": task.updated_at,
            })
        })
        .collect();
    let service_summaries: Vec<_> = services
        .iter()
        .map(|service| {
            json!({
                "name": service.name,
                "baseUrl": service.base_url,
                "proxyMode": service.proxy_mode,
                "maxFileSizeMb": service.max_file_size_mb,
                "maxPages": service.max_pages,
                "qps": service.qps,
                "maxConcurrency": service.max_concurrency,
            })
        })
        .collect();
    let summary = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "settings": {
            "language": settings.language,
            "theme": settings.theme,
            "concurrency": settings.concurrency,
            "retryLimit": settings.retry_limit,
            "historyDays": settings.history_days,
            "logDays": settings.log_days,
        },
        "services": service_summaries,
        "tasks": task_summaries,
    });
    archive
        .start_file("summary.json", options)
        .map_err(|error| error.to_string())?;
    std::io::Write::write_all(
        &mut archive,
        serde_json::to_vec_pretty(&summary)
            .map_err(|error| error.to_string())?
            .as_slice(),
    )
    .map_err(|error| error.to_string())?;

    let logs = data_dir.join("logs");
    if logs.exists() {
        for entry in std::fs::read_dir(logs)
            .map_err(|error| error.to_string())?
            .flatten()
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            archive
                .start_file(format!("logs/{name}"), options)
                .map_err(|error| error.to_string())?;
            std::io::Write::write_all(
                &mut archive,
                &std::fs::read(&path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        }
    }
    archive.finish().map_err(|error| error.to_string())?;
    Ok(destination)
}

fn cleanup_logs(data_dir: &Path, days: u32) {
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(u64::from(days.max(1)) * 24 * 60 * 60);
    let Ok(entries) = std::fs::read_dir(data_dir.join("logs")) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.modified().is_ok_and(|modified| modified < cutoff) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
