//! Audio API wrappers for `MiniMax` (TTS, async TTS, voice operations).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use base64::Engine;
use colored::Colorize;
use serde_json::{Value, json};

use crate::client::MiniMaxClient;
use crate::modules::files::{FileUploadOptions, retrieve_file, upload};
use crate::utils::{
    extension_from_url, output_path, pretty_json, timestamped_filename, write_bytes,
};

// === Types ===

/// Options for synchronous text-to-audio generation.
pub struct T2aOptions {
    pub model: String,
    pub text: String,
    pub stream: bool,
    pub output_format: Option<String>,
    pub voice_id: Option<String>,
    pub voice_setting_json: Option<String>,
    pub audio_setting_json: Option<String>,
    pub pronunciation_dict_json: Option<String>,
    pub timber_weights_json: Option<String>,
    pub language_boost_json: Option<String>,
    pub voice_modify_json: Option<String>,
    pub subtitle_enable: Option<bool>,
    pub output_dir: PathBuf,
}

/// Options for creating an async TTS job.
pub struct T2aAsyncCreateOptions {
    pub model: String,
    pub text: Option<String>,
    pub text_file_id: Option<String>,
    pub voice_id: Option<String>,
    pub voice_setting_json: Option<String>,
    pub audio_setting_json: Option<String>,
    pub pronunciation_dict_json: Option<String>,
    pub language_boost_json: Option<String>,
    pub voice_modify_json: Option<String>,
}

/// Options for querying an async TTS job.
pub struct T2aAsyncQueryOptions {
    pub task_id: String,
}

/// Options for cloning a voice from audio samples.
pub struct VoiceCloneOptions {
    pub clone_audio: PathBuf,
    pub prompt_audio: Option<PathBuf>,
    pub voice_id: Option<String>,
    pub clone_prompt_text: Option<String>,
    pub text: Option<String>,
    pub model: Option<String>,
    pub language_boost_json: Option<String>,
    pub need_noise_reduction: Option<bool>,
    pub need_volume_normalization: Option<bool>,
}

/// Options for listing voices.
pub struct VoiceListOptions {
    pub voice_type: Option<String>,
}

/// Options for deleting a voice.
pub struct VoiceDeleteOptions {
    pub voice_type: Option<String>,
    pub voice_id: String,
}

/// Options for designing a synthetic voice.
pub struct VoiceDesignOptions {
    pub prompt: String,
    pub preview_text: String,
    pub voice_id: Option<String>,
}

// === API Calls ===

pub async fn t2a(client: &MiniMaxClient, options: T2aOptions) -> Result<()> {
    let mut body = json!({
        "model": options.model,
        "text": options.text,
        "stream": options.stream,
    });

    if let Some(output_format) = options.output_format.clone() {
        body["output_format"] = json!(output_format);
    }

    if let Some(voice_id) = options.voice_id {
        body["voice_setting"] = json!({ "voice_id": voice_id });
    }

    merge_json_field(&mut body, "voice_setting", options.voice_setting_json)?;
    merge_json_field(&mut body, "audio_setting", options.audio_setting_json)?;
    merge_json_field(
        &mut body,
        "pronunciation_dict",
        options.pronunciation_dict_json,
    )?;
    merge_json_field(&mut body, "timber_weights", options.timber_weights_json)?;
    merge_json_field(&mut body, "language_boost", options.language_boost_json)?;
    merge_json_field(&mut body, "voice_modify", options.voice_modify_json)?;

    if let Some(subtitle_enable) = options.subtitle_enable {
        body["subtitle_enable"] = json!(subtitle_enable);
    }

    if options.stream {
        let response = client.post_json_raw("/v1/t2a_v2", &body).await?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");

        if content_type.contains("application/json") {
            let value: Value = response.json().await?;
            handle_audio_response(client, &value, &options.output_dir).await?;
        } else {
            let bytes = response.bytes().await?;
            let extension = options
                .output_format
                .clone()
                .unwrap_or_else(|| "wav".to_string());
            let filename = timestamped_filename("speech", &extension);
            let path = output_path(&options.output_dir, &filename);
            write_bytes(&path, &bytes)?;
            println!("{} {}", "Saved".green().bold(), path.display());
        }
    } else {
        let response: Value = client.post_json("/v1/t2a_v2", &body).await?;
        handle_audio_response(client, &response, &options.output_dir).await?;
    }

    Ok(())
}

pub async fn t2a_async_create(
    client: &MiniMaxClient,
    options: T2aAsyncCreateOptions,
) -> Result<Value> {
    let mut body = json!({ "model": options.model });
    if let Some(text) = options.text {
        body["text"] = json!(text);
    }
    if let Some(text_file_id) = options.text_file_id {
        body["text_file_id"] = json!(text_file_id);
    }
    if let Some(voice_id) = options.voice_id {
        body["voice_setting"] = json!({ "voice_id": voice_id });
    }

    merge_json_field(&mut body, "voice_setting", options.voice_setting_json)?;
    merge_json_field(&mut body, "audio_setting", options.audio_setting_json)?;
    merge_json_field(
        &mut body,
        "pronunciation_dict",
        options.pronunciation_dict_json,
    )?;
    merge_json_field(&mut body, "language_boost", options.language_boost_json)?;
    merge_json_field(&mut body, "voice_modify", options.voice_modify_json)?;

    let response: Value = client.post_json("/v1/t2a_async_v2", &body).await?;
    Ok(response)
}

pub async fn t2a_async_query(
    client: &MiniMaxClient,
    options: T2aAsyncQueryOptions,
) -> Result<Value> {
    let response: Value = client
        .get_json(
            "/v1/query/t2a_async_query_v2",
            Some(&[("task_id", options.task_id.as_str())]),
        )
        .await?;
    Ok(response)
}

pub async fn voice_clone(client: &MiniMaxClient, options: VoiceCloneOptions) -> Result<()> {
    let upload_response = upload(
        client,
        FileUploadOptions {
            path: options.clone_audio,
            purpose: "voice_clone".to_string(),
        },
    )
    .await?;

    let clone_file_id = extract_file_id(&upload_response)
        .context("Could not find file_id in voice clone upload response.")?;

    let prompt_audio_id = if let Some(prompt_audio) = options.prompt_audio {
        let prompt_response = upload(
            client,
            FileUploadOptions {
                path: prompt_audio,
                purpose: "prompt_audio".to_string(),
            },
        )
        .await?;
        Some(
            extract_file_id(&prompt_response)
                .context("Could not find file_id in prompt audio upload response.")?,
        )
    } else {
        None
    };

    let mut clone_prompt = serde_json::Map::new();
    if let Some(prompt_audio_id) = prompt_audio_id {
        clone_prompt.insert("prompt_audio".to_string(), json!(prompt_audio_id));
    }
    if let Some(text) = options.clone_prompt_text {
        clone_prompt.insert("text".to_string(), json!(text));
    }

    let mut body = json!({
        "file_id": clone_file_id,
    });
    body["clone_prompt"] = Value::Object(clone_prompt);

    if let Some(voice_id) = options.voice_id {
        body["voice_id"] = json!(voice_id);
    }
    if let Some(text) = options.text {
        body["text"] = json!(text);
    }
    if let Some(model) = options.model {
        body["model"] = json!(model);
    }
    if let Some(language_boost) = options.language_boost_json {
        let parsed: Value = serde_json::from_str(&language_boost)
            .context("Failed to parse language_boost_json: expected JSON.")?;
        body["language_boost"] = parsed;
    }
    if let Some(need_noise_reduction) = options.need_noise_reduction {
        body["need_noise_reduction"] = json!(need_noise_reduction);
    }
    if let Some(need_volume_normalization) = options.need_volume_normalization {
        body["need_volume_normalization"] = json!(need_volume_normalization);
    }

    let response: Value = client.post_json("/v1/voice_clone", &body).await?;
    println!("{}", pretty_json(&response));
    Ok(())
}

pub async fn voice_list(client: &MiniMaxClient, options: VoiceListOptions) -> Result<Value> {
    let mut body = json!({});
    if let Some(voice_type) = options.voice_type {
        body["voice_type"] = json!(voice_type);
    }
    let response: Value = client.post_json("/v1/get_voice", &body).await?;
    Ok(response)
}

pub async fn voice_delete(client: &MiniMaxClient, options: VoiceDeleteOptions) -> Result<Value> {
    let mut body = json!({ "voice_id": options.voice_id });
    if let Some(voice_type) = options.voice_type {
        body["voice_type"] = json!(voice_type);
    }
    let response: Value = client.post_json("/v1/delete_voice", &body).await?;
    Ok(response)
}

pub async fn voice_design(client: &MiniMaxClient, options: VoiceDesignOptions) -> Result<Value> {
    let mut body = json!({
        "prompt": options.prompt,
        "preview_text": options.preview_text,
    });
    if let Some(voice_id) = options.voice_id {
        body["voice_id"] = json!(voice_id);
    }
    let response: Value = client.post_json("/v1/voice_design", &body).await?;
    Ok(response)
}

async fn handle_audio_response(
    client: &MiniMaxClient,
    response: &Value,
    output_dir: &Path,
) -> Result<()> {
    if let Some(url) = extract_audio_url(response) {
        let bytes = client.get_bytes(&url).await?;
        let extension = extension_from_url(&url).unwrap_or_else(|| "wav".to_string());
        let filename = timestamped_filename("audio", &extension);
        let path = output_path(output_dir, &filename);
        write_bytes(&path, &bytes)?;
        println!("{} {}", "Saved".green().bold(), path.display());
        return Ok(());
    }

    if let Some(file_id) = extract_file_id(response) {
        let url_opt = retrieve_file(client, &file_id, Some("audio")).await?;
        if let Some(url) = url_opt {
            let bytes = client.get_bytes(&url).await?;
            let extension = extension_from_url(&url).unwrap_or_else(|| "wav".to_string());
            let filename = timestamped_filename("audio", &extension);
            let path = output_path(output_dir, &filename);
            write_bytes(&path, &bytes)?;
            println!("{} {}", "Saved".green().bold(), path.display());
            return Ok(());
        }
    }

    if let Some(b64) = response
        .get("audio")
        .or_else(|| response.get("audio_base64"))
        .and_then(|value| value.as_str())
    {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .context("Failed to decode audio payload: invalid base64 data.")?;
        let filename = timestamped_filename("audio", "wav");
        let path = output_path(output_dir, &filename);
        write_bytes(&path, &bytes)?;
        println!("{} {}", "Saved".green().bold(), path.display());
        return Ok(());
    }

    println!(
        "{}",
        "Failed to generate audio: no audio payload found in response.".yellow()
    );
    println!("{}", pretty_json(response));
    Ok(())
}

fn extract_audio_url(response: &Value) -> Option<String> {
    if let Some(url) = response
        .get("audio_url")
        .or_else(|| response.get("url"))
        .and_then(|value| value.as_str())
    {
        return Some(url.to_string());
    }
    if let Some(data) = response.get("data")
        && let Some(url) = data
            .get("audio_url")
            .or_else(|| data.get("url"))
            .and_then(|value| value.as_str())
    {
        return Some(url.to_string());
    }
    None
}

fn extract_file_id(response: &Value) -> Option<String> {
    response
        .get("file_id")
        .and_then(|value| value.as_str())
        .map(std::string::ToString::to_string)
        .or_else(|| {
            response
                .get("data")
                .and_then(|value| value.get("file_id"))
                .and_then(|value| value.as_str())
                .map(std::string::ToString::to_string)
        })
}

fn merge_json_field(target: &mut Value, field: &str, raw_json: Option<String>) -> Result<()> {
    if let Some(raw_json) = raw_json {
        let parsed: Value = serde_json::from_str(&raw_json)
            .with_context(|| format!("Failed to parse {field}: expected JSON."))?;
        match target.get_mut(field) {
            Some(existing) => merge_json(existing, parsed),
            None => {
                target[field] = parsed;
            }
        }
    }
    Ok(())
}

fn merge_json(existing: &mut Value, incoming: Value) {
    if let (Some(existing_obj), Some(incoming_obj)) =
        (existing.as_object_mut(), incoming.as_object())
    {
        for (key, value) in incoming_obj {
            existing_obj.insert(key.clone(), value.clone());
        }
    } else {
        *existing = incoming;
    }
}
