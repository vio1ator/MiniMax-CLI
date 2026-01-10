use base64::Engine;
use crate::client::MiniMaxClient;
use crate::modules::files::retrieve_file;
use crate::ui::spinner;
use crate::utils::{extension_from_url, output_path, pretty_json, timestamped_filename, write_bytes};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::time::{sleep, Duration};

pub struct VideoGenerateOptions {
    pub model: String,
    pub prompt: String,
    pub first_frame: Option<String>,
    pub last_frame: Option<String>,
    pub subject_reference: Vec<String>,
    pub subject_reference_json: Option<String>,
    pub duration: Option<u32>,
    pub resolution: Option<String>,
    pub callback_url: Option<String>,
    pub prompt_optimizer: Option<bool>,
    pub fast_pretreatment: Option<bool>,
    pub wait: bool,
    pub output_dir: PathBuf,
}

pub struct VideoQueryOptions {
    pub task_id: String,
}

pub struct VideoAgentCreateOptions {
    pub template_id: String,
    pub text_inputs_json: Option<String>,
    pub media_inputs_json: Option<String>,
    pub callback_url: Option<String>,
}

pub async fn generate(client: &MiniMaxClient, options: VideoGenerateOptions) -> Result<()> {
    let mut body = json!({
        "model": options.model,
        "prompt": options.prompt,
    });

    if let Some(first_frame) = options.first_frame {
        body["first_frame_image"] = json!(resolve_media_input(&first_frame)?);
    }
    if let Some(last_frame) = options.last_frame {
        body["last_frame_image"] = json!(resolve_media_input(&last_frame)?);
    }
    if let Some(duration) = options.duration {
        body["duration"] = json!(duration);
    }
    if let Some(resolution) = options.resolution {
        body["resolution"] = json!(resolution);
    }
    if let Some(callback_url) = options.callback_url {
        body["callback_url"] = json!(callback_url);
    }
    if let Some(prompt_optimizer) = options.prompt_optimizer {
        body["prompt_optimizer"] = json!(prompt_optimizer);
    }
    if let Some(fast_pretreatment) = options.fast_pretreatment {
        body["fast_pretreatment"] = json!(fast_pretreatment);
    }

    if let Some(raw_json) = options.subject_reference_json {
        let parsed: Value = serde_json::from_str(&raw_json)
            .context("Failed to parse --subject-reference-json as JSON.")?;
        body["subject_reference"] = parsed;
    } else if !options.subject_reference.is_empty() {
        let values: Vec<String> = options
            .subject_reference
            .iter()
            .map(|value| resolve_media_input(value))
            .collect::<Result<Vec<_>>>()?;
        body["subject_reference"] = json!(values);
    }

    let response: Value = client.post_json("/v1/video_generation", &body).await?;
    println!("{}", pretty_json(&response));

    if !options.wait {
        return Ok(());
    }

    if let Some(task_id) = extract_task_id(&response) {
        let spinner = spinner("Generating video...");
        let final_response = wait_for_video(client, &task_id).await?;
        spinner.finish_and_clear();
        println!("{}", pretty_json(&final_response));
        if let Some(path) = download_video_if_available(client, &final_response, &options.output_dir).await? {
            println!("{} {}", "Saved".green().bold(), path.display());
        }
    } else if let Some(path) = download_video_if_available(client, &response, &options.output_dir).await? {
        println!("{} {}", "Saved".green().bold(), path.display());
    }

    Ok(())
}

pub async fn query(client: &MiniMaxClient, options: VideoQueryOptions) -> Result<()> {
    let response: Value = client
        .get_json("/v1/query/video_generation", Some(&[("task_id", options.task_id.as_str())]))
        .await?;
    println!("{}", pretty_json(&response));
    Ok(())
}

pub async fn agent_create(client: &MiniMaxClient, options: VideoAgentCreateOptions) -> Result<()> {
    let mut body = json!({ "template_id": options.template_id });

    if let Some(text_inputs) = options.text_inputs_json {
        let parsed: Value = serde_json::from_str(&text_inputs)
            .context("Failed to parse --text-inputs-json as JSON.")?;
        body["text_inputs"] = parsed;
    }
    if let Some(media_inputs) = options.media_inputs_json {
        let parsed: Value = serde_json::from_str(&media_inputs)
            .context("Failed to parse --media-inputs-json as JSON.")?;
        body["media_inputs"] = parsed;
    }
    if let Some(callback_url) = options.callback_url {
        body["callback_url"] = json!(callback_url);
    }

    let response: Value = client.post_json("/v1/video_template_generation", &body).await?;
    println!("{}", pretty_json(&response));
    Ok(())
}

pub async fn agent_query(client: &MiniMaxClient, task_id: &str) -> Result<()> {
    let response: Value = client
        .get_json(
            "/v1/query/video_template_generation",
            Some(&[("task_id", task_id)]),
        )
        .await?;
    println!("{}", pretty_json(&response));
    Ok(())
}

async fn wait_for_video(client: &MiniMaxClient, task_id: &str) -> Result<Value> {
    loop {
        let response: Value = client
            .get_json("/v1/query/video_generation", Some(&[("task_id", task_id)]))
            .await?;

        if let Some(status) = extract_status(&response) {
            let status_upper = status.to_uppercase();
            if status_upper.contains("SUCCESS")
                || status_upper.contains("FINISH")
                || status_upper.contains("DONE")
            {
                return Ok(response);
            }
            if status_upper.contains("FAIL") || status_upper.contains("ERROR") {
                return Ok(response);
            }
        }

        sleep(Duration::from_secs(3)).await;
    }
}

async fn download_video_if_available(
    client: &MiniMaxClient,
    response: &Value,
    output_dir: &Path,
) -> Result<Option<PathBuf>> {
    if let Some(url) = extract_video_url(response) {
        let bytes = client.get_bytes(&url).await?;
        let extension = extension_from_url(&url).unwrap_or_else(|| "mp4".to_string());
        let filename = timestamped_filename("video", &extension);
        let path = output_path(output_dir, &filename);
        write_bytes(&path, &bytes)?;
        return Ok(Some(path));
    }

    if let Some(file_id) = extract_file_id(response) {
        if let Some(url) = retrieve_file(client, &file_id, Some("video")).await? {
            let bytes = client.get_bytes(&url).await?;
            let extension = extension_from_url(&url).unwrap_or_else(|| "mp4".to_string());
            let filename = timestamped_filename("video", &extension);
            let path = output_path(output_dir, &filename);
            write_bytes(&path, &bytes)?;
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn extract_task_id(response: &Value) -> Option<String> {
    response
        .get("task_id")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .or_else(|| {
            response
                .get("data")
                .and_then(|v| v.get("task_id"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
        })
}

fn extract_status(response: &Value) -> Option<String> {
    response
        .get("status")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .or_else(|| {
            response
                .get("data")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
        })
}

fn extract_video_url(response: &Value) -> Option<String> {
    if let Some(url) = response
        .get("video_url")
        .or_else(|| response.get("url"))
        .and_then(|value| value.as_str())
    {
        return Some(url.to_string());
    }

    if let Some(data) = response.get("data") {
        if let Some(url) = data
            .get("video_url")
            .or_else(|| data.get("url"))
            .and_then(|value| value.as_str())
        {
            return Some(url.to_string());
        }
    }

    None
}

fn extract_file_id(response: &Value) -> Option<String> {
    response
        .get("file_id")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| {
            response
                .get("data")
                .and_then(|value| value.get("file_id"))
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
}

fn resolve_media_input(value: &str) -> Result<String> {
    let path = Path::new(value);
    if !path.exists() {
        return Ok(value.to_string());
    }

    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read media file: {}", path.display()))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mime = match path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_lowercase().as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    };
    Ok(format!("data:{};base64,{}", mime, encoded))
}
