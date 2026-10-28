use crate::models::{
    ApiEnvelope, AppError, OfficeData, ParseData, ProxyMode, ServiceProfile, Task, TaskSource,
    Usage,
};
use futures_util::StreamExt;
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;
pub type ApiResult<T> = Result<T, AppError>;

#[derive(Clone)]
pub struct ApiClient {
    requests: Arc<Mutex<HashMap<String, Instant>>>,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn client(&self, profile: &ServiceProfile) -> ApiResult<Client> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(profile.timeout_secs))
            .user_agent(concat!("SoMarkClient-CE/", env!("CARGO_PKG_VERSION")));
        builder = match profile.proxy_mode {
            ProxyMode::System => builder,
            ProxyMode::Direct => builder.no_proxy(),
            ProxyMode::Custom => {
                let proxy_url = profile
                    .proxy_url
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| AppError::new("proxy", "Custom proxy URL is required"))?;
                let proxy = reqwest::Proxy::all(proxy_url)
                    .map_err(|_| AppError::new("proxy", "The custom proxy URL is invalid"))?;
                builder.proxy(proxy)
            }
        };
        builder
            .build()
            .map_err(|_| AppError::new("network", "Unable to initialize the HTTP client"))
    }

    async fn throttle(&self, profile: &ServiceProfile) {
        let interval = Duration::from_secs_f64(1.0 / profile.qps.max(1) as f64);
        let mut requests = self.requests.lock().await;
        if let Some(previous) = requests.get(&profile.id) {
            let elapsed = previous.elapsed();
            if elapsed < interval {
                tokio::time::sleep(interval - elapsed).await;
            }
        }
        requests.insert(profile.id.clone(), Instant::now());
    }

    async fn decode<T: DeserializeOwned>(response: Response) -> ApiResult<(T, Vec<String>)> {
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| map_request_error(&error))?;
        let envelope: ApiEnvelope<T> = serde_json::from_slice(&bytes).map_err(|_| {
            if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
                AppError::network(format!("The service returned HTTP {status}"))
            } else {
                AppError::new("response", format!("Invalid API response (HTTP {status})"))
            }
        })?;
        if envelope.code != 0 {
            return Err(AppError::api(envelope.code, envelope.message));
        }
        if !status.is_success() {
            return Err(AppError::new(
                "http",
                format!("The service returned HTTP {status}"),
            ));
        }
        let data = envelope
            .data
            .ok_or_else(|| AppError::new("response", "The API response did not contain data"))?;
        Ok((data, envelope.warnings))
    }

    async fn post<T: DeserializeOwned>(
        &self,
        profile: &ServiceProfile,
        path: &str,
        form: Form,
    ) -> ApiResult<(T, Vec<String>)> {
        self.throttle(profile).await;
        let response = self
            .client(profile)?
            .post(profile.endpoint(path))
            .multipart(form)
            .send()
            .await
            .map_err(|error| map_request_error(&error))?;
        Self::decode(response).await
    }

    pub async fn usage(&self, profile: &ServiceProfile, key: &str) -> ApiResult<Usage> {
        let form = Form::new().text("api_key", key.to_owned());
        let (mut usage, warnings): (Usage, Vec<String>) =
            self.post(profile, "/usage", form).await?;
        usage.warnings = warnings;
        Ok(usage)
    }

    async fn parse_form(
        &self,
        _profile: &ServiceProfile,
        key: &str,
        task: &Task,
        progress: Option<ProgressCallback>,
    ) -> ApiResult<Form> {
        let mut form = Form::new().text("api_key", key.to_owned());
        for format in &task.formats {
            let format = if format == "md" { "markdown" } else { format };
            form = form.text("output_formats", format.to_owned());
        }
        form = form
            .text(
                "element_formats",
                serde_json::to_string(&task.parse_options.element_formats)
                    .map_err(|_| AppError::new("parameters", "Invalid element formats"))?,
            )
            .text(
                "feature_config",
                serde_json::to_string(&task.parse_options.feature_config)
                    .map_err(|_| AppError::new("parameters", "Invalid feature config"))?,
            );
        self.add_source(form, &task.source, progress).await
    }

    async fn office_form(
        &self,
        key: &str,
        task: &Task,
        progress: Option<ProgressCallback>,
    ) -> ApiResult<Form> {
        let form = Form::new()
            .text("api_key", key.to_owned())
            .text("target_type", task.office_options.target_type.clone())
            .text("layout_mode", task.office_options.layout_mode.clone());
        self.add_source(form, &task.source, progress).await
    }

    async fn add_source(
        &self,
        form: Form,
        source: &TaskSource,
        progress: Option<ProgressCallback>,
    ) -> ApiResult<Form> {
        match source {
            TaskSource::Url { url } => Ok(form.text("file_url", url.clone())),
            TaskSource::Local { path } | TaskSource::Clipboard { path } => {
                let part = streaming_file_part(Path::new(path), progress).await?;
                Ok(form.part("file", part))
            }
        }
    }

    pub async fn parse_sync(
        &self,
        profile: &ServiceProfile,
        key: &str,
        task: &Task,
        progress: ProgressCallback,
    ) -> ApiResult<(ParseData, Vec<String>)> {
        let form = self.parse_form(profile, key, task, Some(progress)).await?;
        self.post(profile, "/parse/sync", form).await
    }

    pub async fn parse_submit(
        &self,
        profile: &ServiceProfile,
        key: &str,
        task: &Task,
        progress: ProgressCallback,
    ) -> ApiResult<(ParseData, Vec<String>)> {
        let form = self.parse_form(profile, key, task, Some(progress)).await?;
        self.post(profile, "/parse/async", form).await
    }

    pub async fn parse_check(
        &self,
        profile: &ServiceProfile,
        key: &str,
        remote_id: &str,
    ) -> ApiResult<(ParseData, Vec<String>)> {
        let form = Form::new()
            .text("api_key", key.to_owned())
            .text("task_id", remote_id.to_owned());
        self.post(profile, "/parse/async_check", form).await
    }

    pub async fn office_sync(
        &self,
        profile: &ServiceProfile,
        key: &str,
        task: &Task,
        progress: ProgressCallback,
    ) -> ApiResult<(OfficeData, Vec<String>)> {
        let form = self.office_form(key, task, Some(progress)).await?;
        self.post(profile, "/convert/sync", form).await
    }

    pub async fn office_submit(
        &self,
        profile: &ServiceProfile,
        key: &str,
        task: &Task,
        progress: ProgressCallback,
    ) -> ApiResult<(OfficeData, Vec<String>)> {
        let form = self.office_form(key, task, Some(progress)).await?;
        self.post(profile, "/convert/async", form).await
    }

    pub async fn office_check(
        &self,
        profile: &ServiceProfile,
        key: &str,
        remote_id: &str,
    ) -> ApiResult<(OfficeData, Vec<String>)> {
        let form = Form::new()
            .text("api_key", key.to_owned())
            .text("task_id", remote_id.to_owned());
        self.post(profile, "/convert/async_check", form).await
    }

    pub async fn download(
        &self,
        profile: &ServiceProfile,
        url: &str,
        destination: &Path,
        progress: ProgressCallback,
    ) -> ApiResult<PathBuf> {
        let parsed = url::Url::parse(url)
            .map_err(|_| AppError::new("download", "The result download URL is invalid"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(AppError::new("download", "Unsupported download URL scheme"));
        }
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|_| AppError::new("download", "Unable to create the output directory"))?;
        }
        let temporary = destination.with_extension(format!(
            "{}.part",
            destination
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
        ));
        let response = self
            .client(profile)?
            .get(url)
            .send()
            .await
            .map_err(|error| map_request_error(&error))?;
        if !response.status().is_success() {
            return Err(AppError::network(format!(
                "The result download returned HTTP {}",
                response.status()
            )));
        }
        let total = response.content_length().unwrap_or(0);
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(&temporary)
            .await
            .map_err(|_| AppError::new("download", "Unable to create the temporary output file"))?;
        let mut written = 0_u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| map_request_error(&error))?;
            file.write_all(&chunk)
                .await
                .map_err(|_| AppError::new("download", "Unable to write the output file"))?;
            written += chunk.len() as u64;
            progress(written, total);
        }
        file.flush()
            .await
            .map_err(|_| AppError::new("download", "Unable to flush the output file"))?;
        drop(file);
        if destination.exists() {
            tokio::fs::remove_file(destination)
                .await
                .map_err(|_| AppError::new("download", "Unable to replace the existing result"))?;
        }
        tokio::fs::rename(&temporary, destination)
            .await
            .map_err(|_| AppError::new("download", "Unable to finalize the output file"))?;
        Ok(destination.to_path_buf())
    }
}

fn map_request_error(error: &reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::timeout("The request timed out")
    } else {
        AppError::network("The network request failed")
    }
}

async fn streaming_file_part(path: &Path, progress: Option<ProgressCallback>) -> ApiResult<Part> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|_| AppError::new("file", "Unable to open the source file"))?;
    let size = file
        .metadata()
        .await
        .map_err(|_| AppError::new("file", "Unable to inspect the source file"))?
        .len();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::new("file", "The source file name is invalid"))?
        .to_owned();
    let stream: std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>,
    > = Box::pin(async_stream::try_stream! {
        let mut sent = 0_u64;
        let mut buffer = vec![0_u8; 256 * 1024];
        loop {
            let read = file.read(&mut buffer).await.map_err(std::io::Error::other)?;
            if read == 0 {
                break;
            }
            sent += read as u64;
            if let Some(callback) = &progress {
                callback(sent, size);
            }
            yield bytes::Bytes::copy_from_slice(&buffer[..read]);
        }
    });
    Part::stream_with_length(Body::wrap_stream(stream), size)
        .file_name(name)
        .mime_str("application/octet-stream")
        .map_err(|_| AppError::new("file", "Unable to construct the file upload"))
}
