use crate::api::{ApiClient, ProgressCallback};
use crate::models::{AppError, OfficeData, OutputFile, ParseResult, ServiceProfile, Task};
use crate::store::atomic_write;
use std::path::{Path, PathBuf};

pub fn output_directory(task: &Task) -> PathBuf {
    if let Some(parent) = task
        .outputs
        .first()
        .and_then(|output| Path::new(&output.path).parent())
    {
        return parent.to_path_buf();
    }
    let stem = task
        .source
        .path()
        .and_then(|path| path.file_stem())
        .and_then(|value| value.to_str())
        .unwrap_or(&task.name);
    let stem = sanitize_filename::sanitize(stem);
    let stem = if stem.trim().is_empty() {
        "document"
    } else {
        &stem
    };
    unique_path(Path::new(&task.output_root).join(stem), true)
}

pub async fn persist_parse(
    task: &Task,
    result: &ParseResult,
    api: &ApiClient,
    profile: &ServiceProfile,
    progress: ProgressCallback,
) -> (Vec<OutputFile>, Option<AppError>) {
    let directory = output_directory(task);
    let stem = directory
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let mut outputs = task.outputs.clone();
    let mut failure = None;

    if task
        .formats
        .iter()
        .any(|value| value == "md" || value == "markdown")
        && !outputs.iter().any(|value| value.format == "md")
    {
        match result.outputs.markdown.as_ref() {
            Some(markdown) => match save_bytes(&directory, stem, "md", markdown.as_bytes()) {
                Ok(path) => outputs.push(output("md", path)),
                Err(error) => failure = Some(error),
            },
            None => failure = Some(AppError::new("result", "Markdown output is missing")),
        }
    }

    if task.formats.iter().any(|value| value == "json")
        && !outputs.iter().any(|value| value.format == "json")
    {
        match result.outputs.json_output.as_ref() {
            Some(json) => match serde_json::to_vec_pretty(json) {
                Ok(bytes) => match save_bytes(&directory, stem, "json", &bytes) {
                    Ok(path) => outputs.push(output("json", path)),
                    Err(error) => failure = Some(error),
                },
                Err(_) => {
                    failure = Some(AppError::new("result", "Unable to serialize JSON output"))
                }
            },
            None => failure = Some(AppError::new("result", "JSON output is missing")),
        }
    }

    if task.formats.iter().any(|value| value == "zip")
        && !outputs.iter().any(|value| value.format == "zip")
    {
        match result.outputs.zip.as_ref() {
            Some(url) => {
                let destination = unique_path(directory.join(format!("{stem}.zip")), false);
                match api.download(profile, url, &destination, progress).await {
                    Ok(path) => outputs.push(output("zip", path)),
                    Err(error) => failure = Some(error),
                }
            }
            None => failure = Some(AppError::new("result", "ZIP output is missing")),
        }
    }

    (outputs, failure)
}

pub async fn persist_office(
    task: &Task,
    result: &OfficeData,
    api: &ApiClient,
    profile: &ServiceProfile,
    progress: ProgressCallback,
) -> (Vec<OutputFile>, Option<AppError>) {
    if !task.outputs.is_empty() {
        return (task.outputs.clone(), None);
    }
    let url = match result.download_url.as_ref() {
        Some(url) => url,
        None => {
            return (
                Vec::new(),
                Some(AppError::new("result", "Office download URL is missing")),
            )
        }
    };
    let directory = output_directory(task);
    let extension = if task.office_options.target_type == "ppt" {
        "pptx"
    } else {
        "docx"
    };
    let stem = directory
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let destination = unique_path(directory.join(format!("{stem}.{extension}")), false);
    match api.download(profile, url, &destination, progress).await {
        Ok(path) => (vec![output(extension, path)], None),
        Err(error) => (task.outputs.clone(), Some(error)),
    }
}

pub fn save_bytes(
    root: &Path,
    stem: &str,
    extension: &str,
    bytes: &[u8],
) -> Result<PathBuf, AppError> {
    let path = unique_path(root.join(format!("{stem}.{extension}")), false);
    atomic_write(&path, bytes)
        .map_err(|_| AppError::new("output", format!("Unable to save {extension} output")))?;
    Ok(path)
}

pub fn unique_path(path: PathBuf, directory: bool) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let extension = if directory {
        None
    } else {
        path.extension().and_then(|value| value.to_str())
    };
    for index in 1..10_000 {
        let name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}

fn output(format: impl Into<String>, path: PathBuf) -> OutputFile {
    OutputFile {
        format: format.into(),
        path: path.to_string_lossy().into_owned(),
    }
}
