use crate::client::MiniMaxClient;
use crate::ui::progress_bar;
use crate::utils::{output_path, pretty_json, write_bytes};
use anyhow::{Context, Result};
use colored::Colorize;
use futures_util::TryStreamExt;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio_util::io::ReaderStream;

pub struct FileUploadOptions {
    pub path: PathBuf,
    pub purpose: String,
}

pub struct FileListOptions {
    pub purpose: String,
}

pub struct FileDeleteOptions {
    pub file_id: String,
    pub purpose: Option<String>,
}

pub struct FileRetrieveOptions {
    pub file_id: String,
    pub purpose: Option<String>,
}

pub struct FileRetrieveContentOptions {
    pub file_id: String,
    pub output: Option<PathBuf>,
    pub output_dir: PathBuf,
}

pub async fn upload(client: &MiniMaxClient, options: FileUploadOptions) -> Result<Value> {
    let file = tokio::fs::File::open(&options.path)
        .await
        .with_context(|| format!("Failed to open {}", options.path.display()))?;
    let metadata = file.metadata().await?;
    let total = metadata.len();
    let filename = options
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("upload.bin")
        .to_string();

    let progress = progress_bar(total, "Uploading file...");
    let progress_clone = progress.clone();
    let stream = ReaderStream::new(file).map_ok(move |chunk| {
        progress_clone.inc(chunk.len() as u64);
        chunk
    });
    let body = reqwest::Body::wrap_stream(stream);
    let part = reqwest::multipart::Part::stream_with_length(body, total).file_name(filename);
    let form = reqwest::multipart::Form::new()
        .text("purpose", options.purpose)
        .part("file", part);

    let response: Value = client.post_multipart("/v1/files/upload", form).await?;
    progress.finish_and_clear();
    println!("{}", pretty_json(&response));
    Ok(response)
}

pub async fn list(client: &MiniMaxClient, options: FileListOptions) -> Result<()> {
    let response: Value = client
        .get_json("/v1/files/list", Some(&[("purpose", options.purpose.as_str())]))
        .await?;
    println!("{}", pretty_json(&response));
    Ok(())
}

pub async fn delete(client: &MiniMaxClient, options: FileDeleteOptions) -> Result<()> {
    let mut body = json!({ "file_id": options.file_id });
    if let Some(purpose) = options.purpose {
        body["purpose"] = json!(purpose);
    }
    let response: Value = client.post_json("/v1/files/delete", &body).await?;
    println!("{}", pretty_json(&response));
    Ok(())
}

pub async fn retrieve(client: &MiniMaxClient, options: FileRetrieveOptions) -> Result<()> {
    let url = retrieve_file(client, &options.file_id, options.purpose.as_deref()).await?;
    if let Some(url) = url {
        println!("{}", url);
    }
    Ok(())
}

pub async fn retrieve_content(
    client: &MiniMaxClient,
    options: FileRetrieveContentOptions,
) -> Result<()> {
    let (bytes, content_type) = client
        .get_bytes_with_query(
            "/v1/files/retrieve_content",
            &[("file_id", options.file_id.as_str())],
        )
        .await?;

    let extension = content_type
        .as_deref()
        .and_then(extension_from_content_type)
        .unwrap_or("bin");

    let filename = format!("file_{}.{}", options.file_id, extension);
    let path = options
        .output
        .unwrap_or_else(|| output_path(&options.output_dir, &filename));
    write_bytes(&path, &bytes)?;
    println!("Saved {}", path.display());
    Ok(())
}

pub async fn retrieve_file(
    client: &MiniMaxClient,
    file_id: &str,
    purpose: Option<&str>,
) -> Result<Option<String>> {
    let mut body = json!({ "file_id": file_id });
    if let Some(purpose) = purpose {
        body["purpose"] = json!(purpose);
    }
    let response: Value = client.post_json("/v1/files/retrieve", &body).await?;

    if let Some(url) = response
        .get("file_url")
        .or_else(|| response.get("url"))
        .and_then(|value| value.as_str())
    {
        return Ok(Some(url.to_string()));
    }
    if let Some(data) = response.get("data") {
        if let Some(url) = data
            .get("file_url")
            .or_else(|| data.get("url"))
            .and_then(|value| value.as_str())
        {
            return Ok(Some(url.to_string()));
        }
    }

    println!("{}", "No file URL returned from retrieve endpoint.".yellow());
    println!("{}", pretty_json(&response));
    Ok(None)
}

fn extension_from_content_type(content_type: &str) -> Option<&'static str> {
    let normalized = content_type.to_lowercase();
    if normalized.contains("pdf") {
        return Some("pdf");
    }
    if normalized.contains("msword") {
        return Some("doc");
    }
    if normalized.contains("officedocument.wordprocessingml") {
        return Some("docx");
    }
    if normalized.contains("json") {
        return Some("jsonl");
    }
    if normalized.contains("text/plain") {
        return Some("txt");
    }
    if normalized.contains("audio/mpeg") {
        return Some("mp3");
    }
    if normalized.contains("audio/wav") || normalized.contains("audio/wave") {
        return Some("wav");
    }
    if normalized.contains("audio/mp4") || normalized.contains("audio/x-m4a") {
        return Some("m4a");
    }
    None
}
