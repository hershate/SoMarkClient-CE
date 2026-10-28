use crate::models::{Task, TaskSource};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub fn prepare(app: &AppHandle, task: &Task) -> Result<(), String> {
    let scope = app.asset_protocol_scope();
    if let TaskSource::Local { path } | TaskSource::Clipboard { path } = &task.source {
        scope.allow_file(path).map_err(|error| error.to_string())?;
    }
    for output in &task.outputs {
        scope
            .allow_file(&output.path)
            .map_err(|error| error.to_string())?;
        if output.format == "zip" {
            if let Some(parent) = Path::new(&output.path).parent() {
                let resources = parent.join("assets");
                if resources.exists() {
                    scope
                        .allow_directory(&resources, true)
                        .map_err(|error| error.to_string())?;
                }
            }
        }
    }
    Ok(())
}

pub fn resolve(app: &AppHandle, task: &Task, source: &str) -> Result<String, String> {
    let relative = source.trim_start_matches("./").trim_start_matches('/');
    if relative.is_empty() || relative.contains("..") || Path::new(relative).is_absolute() {
        return Err("Unsafe resource path".into());
    }
    for output in &task.outputs {
        if output.format != "zip" {
            continue;
        }
        let Some(parent) = Path::new(&output.path).parent() else {
            continue;
        };
        let root = parent.join("assets");
        let candidate = root.join(relative);
        if candidate.is_file() {
            return allow_resource(app, &root, &candidate);
        }
        let basename = Path::new(relative)
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "Invalid resource name".to_string())?;
        if let Some(found) = find_by_name(&root, basename, 0) {
            return allow_resource(app, &root, &found);
        }
    }
    Err("Resource is not present in the downloaded ZIP".into())
}

fn allow_resource(app: &AppHandle, root: &Path, path: &Path) -> Result<String, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    if !path.starts_with(&root) {
        return Err("Unsafe resource path".into());
    }
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

fn find_by_name(root: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    if depth > 12 {
        return None;
    }
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_by_name(&path, name, depth + 1) {
                return Some(found);
            }
        } else if path.file_name().and_then(|value| value.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}
