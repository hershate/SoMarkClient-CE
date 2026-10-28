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
    result: &mut ParseResult,
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

    if let Some(zip) = outputs.iter().find(|value| value.format == "zip") {
        if let Err(error) = extract_zip_resources(Path::new(&zip.path), &directory.join("assets")) {
            failure = Some(error);
        }
    }

    let wants_markdown = task
        .formats
        .iter()
        .any(|value| value == "md" || value == "markdown");
    if wants_markdown
        && result.outputs.markdown.is_none()
        && outputs.iter().any(|value| value.format == "zip")
    {
        match markdown_from_zip(&directory.join("assets")) {
            Ok(markdown) => result.outputs.markdown = Some(markdown),
            Err(error) if failure.is_none() => failure = Some(error),
            _ => {}
        }
    }

    if wants_markdown && !outputs.iter().any(|value| value.format == "md") {
        match result.outputs.markdown.as_ref() {
            Some(markdown) => match save_bytes(&directory, stem, "md", markdown.as_bytes()) {
                Ok(path) => outputs.push(output("md", path)),
                Err(error) => failure = Some(error),
            },
            None if failure.is_none() => {
                failure = Some(AppError::new("result", "Markdown output is missing"))
            }
            _ => {}
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

pub fn save_markdown_copy(
    task: &Task,
    content: &str,
    destination: Option<String>,
) -> Result<OutputFile, AppError> {
    let directory = output_directory(task);
    let stem = directory
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let path = destination
        .map(PathBuf::from)
        .unwrap_or_else(|| directory.join(format!("{stem}.edited.md")));
    let path = unique_path(path, false);
    atomic_write(&path, content.as_bytes())
        .map_err(|_| AppError::new("output", "Unable to save the Markdown copy"))?;
    Ok(output("md", path))
}

pub fn export_parse(
    task: &Task,
    result: &ParseResult,
    format: &str,
) -> Result<OutputFile, AppError> {
    let directory = output_directory(task);
    let stem = directory
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    match format {
        "html" => {
            let markdown = result.outputs.markdown.as_deref().ok_or_else(|| {
                AppError::new("export", "Markdown output is required for HTML export")
            })?;
            let mut options = pulldown_cmark::Options::empty();
            options.insert(pulldown_cmark::Options::ENABLE_TABLES);
            options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
            options.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
            options.insert(pulldown_cmark::Options::ENABLE_FOOTNOTES);
            let parser = pulldown_cmark::Parser::new_ext(markdown, options);
            let mut body = String::new();
            pulldown_cmark::html::push_html(&mut body, parser);
            let body = ammonia::clean(&body);
            let document = format!(
                "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>{}</title><style>body{{max-width:960px;margin:40px auto;padding:0 24px;font:16px/1.75 system-ui,sans-serif;color:#17201d}}img{{max-width:100%}}table{{width:100%;border-collapse:collapse}}th,td{{padding:8px;border:1px solid #ddd}}pre{{overflow:auto;padding:16px;background:#f5f6f3}}</style></head><body>{}</body></html>",
                sanitize_filename::sanitize(stem),
                body
            );
            let path = save_bytes(&directory, stem, "html", document.as_bytes())?;
            Ok(output("html", path))
        }
        "txt" => {
            let text = result
                .outputs
                .json_output
                .as_ref()
                .map(json_to_text)
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    result
                        .outputs
                        .markdown
                        .as_ref()
                        .map(|value| markdown_to_text(value))
                })
                .ok_or_else(|| AppError::new("export", "No text output is available"))?;
            let path = save_bytes(&directory, stem, "txt", text.as_bytes())?;
            Ok(output("txt", path))
        }
        _ => Err(AppError::new("export", "Unsupported export format")),
    }
}

fn json_to_text(value: &serde_json::Value) -> String {
    let mut output = String::new();
    if let Some(pages) = value.get("pages").and_then(|value| value.as_array()) {
        for page in pages {
            if let Some(blocks) = page.get("blocks").and_then(|value| value.as_array()) {
                for block in blocks {
                    if let Some(content) = block.get("content").and_then(|value| value.as_str()) {
                        if !output.is_empty() {
                            output.push_str("\n\n");
                        }
                        output.push_str(content);
                    }
                }
            }
        }
    }
    output
}

fn markdown_to_text(markdown: &str) -> String {
    let mut output = String::new();
    for event in pulldown_cmark::Parser::new(markdown) {
        match event {
            pulldown_cmark::Event::Text(text) | pulldown_cmark::Event::Code(text) => {
                output.push_str(&text)
            }
            pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => {
                output.push('\n')
            }
            pulldown_cmark::Event::End(
                pulldown_cmark::TagEnd::Paragraph
                | pulldown_cmark::TagEnd::Heading(_)
                | pulldown_cmark::TagEnd::Item,
            ) => output.push_str("\n\n"),
            _ => {}
        }
    }
    output
}

fn markdown_from_zip(root: &Path) -> Result<String, AppError> {
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files, 0)?;
    files.sort();
    let path = files
        .iter()
        .find(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("main.md"))
        })
        .or_else(|| files.first())
        .ok_or_else(|| AppError::new("result", "The ZIP does not contain Markdown output"))?;
    std::fs::read_to_string(path)
        .map_err(|_| AppError::new("result", "Unable to read Markdown from the ZIP"))
}

fn collect_markdown_files(
    root: &Path,
    files: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), AppError> {
    if depth > 12 || !root.exists() {
        return Ok(());
    }
    let entries = std::fs::read_dir(root)
        .map_err(|_| AppError::new("resources", "Unable to inspect extracted resources"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files, depth + 1)?;
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "md" | "markdown" | "smd"
                )
            })
        {
            files.push(path);
        }
    }
    Ok(())
}

fn extract_zip_resources(zip_path: &Path, destination: &Path) -> Result<(), AppError> {
    let file = std::fs::File::open(zip_path)
        .map_err(|_| AppError::new("resources", "Unable to open the downloaded ZIP"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|_| AppError::new("resources", "The downloaded ZIP is invalid"))?;
    if archive.len() > 10_000 {
        return Err(AppError::new(
            "resources",
            "The ZIP contains too many entries",
        ));
    }
    let mut total_size = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| AppError::new("resources", "Unable to read a ZIP entry"))?;
        total_size = total_size.saturating_add(entry.size());
        if total_size > 2 * 1024 * 1024 * 1024 {
            return Err(AppError::new(
                "resources",
                "The ZIP expands beyond the safety limit",
            ));
        }
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| AppError::new("resources", "The ZIP contains an unsafe path"))?
            .to_path_buf();
        let path = destination.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&path)
                .map_err(|_| AppError::new("resources", "Unable to create a resource directory"))?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| AppError::new("resources", "Unable to create a resource directory"))?;
        }
        let mut output = std::fs::File::create(&path)
            .map_err(|_| AppError::new("resources", "Unable to write an extracted resource"))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|_| AppError::new("resources", "Unable to extract a ZIP resource"))?;
    }
    Ok(())
}
