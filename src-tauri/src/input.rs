use crate::models::{PreflightResult, ServiceProfile};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "bmp", "tiff", "jp2", "dib", "ppm", "pgm", "pbm", "gif", "heic",
    "heif", "webp", "xpm", "tga", "dds", "xbm", "doc", "docx", "ppt", "pptx", "xlsx", "xlsm",
    "xls",
];

pub fn preflight_paths(
    paths: &[String],
    recursive: bool,
    profile: &ServiceProfile,
) -> Vec<PreflightResult> {
    let mut files = Vec::new();
    for value in paths {
        collect_files(Path::new(value), recursive, &mut files);
        if files.len() >= 1000 {
            break;
        }
    }
    files.truncate(1000);
    files
        .iter()
        .map(|path| preflight_file(path, profile))
        .collect()
}

pub fn preflight_file(path: &Path, profile: &ServiceProfile) -> PreflightResult {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document")
        .to_string();
    let mut result = PreflightResult {
        name,
        path: path.to_string_lossy().into_owned(),
        size: 0,
        page_count: None,
        accepted: false,
        issues: Vec::new(),
    };
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();
    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        result.issues.push("unsupportedFileType".into());
    }
    let metadata = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => {
            result.issues.push("fileNotReadable".into());
            return result;
        }
    };
    result.size = metadata.len();
    if result.size == 0 {
        result.issues.push("emptyFile".into());
    }
    if result.size > profile.max_file_size_mb.saturating_mul(1024 * 1024) {
        result.issues.push("fileTooLarge".into());
    }
    if fs::File::open(path).is_err() {
        result.issues.push("fileNotReadable".into());
    }
    if extension == "pdf" && result.size > 0 {
        match lopdf::Document::load(path) {
            Ok(document) => {
                if document.trailer.get(b"Encrypt").is_ok() {
                    result.issues.push("encryptedPdf".into());
                }
                let pages = document.get_pages().len() as u32;
                result.page_count = Some(pages);
                if pages > profile.max_pages {
                    result.issues.push("tooManyPages".into());
                }
            }
            Err(_) => result.issues.push("damagedOrEncryptedPdf".into()),
        }
    }
    result.issues.sort();
    result.issues.dedup();
    result.accepted = result.issues.is_empty();
    result
}

pub fn import_clipboard(
    data_dir: &Path,
    profile: &ServiceProfile,
) -> Result<PreflightResult, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|_| "clipboardUnavailable".to_string())?;
    let image = clipboard
        .get_image()
        .map_err(|_| "clipboardHasNoImage".to_string())?;
    let width = u32::try_from(image.width).map_err(|_| "clipboardImageInvalid".to_string())?;
    let height = u32::try_from(image.height).map_err(|_| "clipboardImageInvalid".to_string())?;
    let directory = data_dir.join("clipboard");
    fs::create_dir_all(&directory).map_err(|_| "clipboardImageSaveFailed".to_string())?;
    let path = directory.join(format!("clipboard-{}.png", Uuid::new_v4()));
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|_| "clipboardImageSaveFailed".to_string())?;
        writer
            .write_image_data(&image.bytes)
            .map_err(|_| "clipboardImageSaveFailed".to_string())?;
    }
    fs::write(&path, bytes).map_err(|_| "clipboardImageSaveFailed".to_string())?;
    Ok(preflight_file(&path, profile))
}

fn collect_files(path: &Path, recursive: bool, output: &mut Vec<PathBuf>) {
    if path.is_file() {
        output.push(path.to_path_buf());
        return;
    }
    if !path.is_dir() {
        output.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        output.push(path.to_path_buf());
        return;
    };
    for entry in entries.flatten() {
        let child = entry.path();
        if child.is_file() {
            output.push(child);
        } else if recursive && child.is_dir() {
            collect_files(&child, true, output);
        }
        if output.len() >= 1000 {
            return;
        }
    }
}
