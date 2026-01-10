use base64::Engine;
use crate::client::MiniMaxClient;
use crate::utils::{extension_from_url, output_path, pretty_json, timestamped_filename, write_bytes};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct MusicGenerateOptions {
    pub model: String,
    pub prompt: String,
    pub lyrics: Option<String>,
    pub stream: bool,
    pub output_format: Option<String>,
    pub audio_setting_json: Option<String>,
    pub output_dir: PathBuf,
}

pub async fn generate(client: &MiniMaxClient, options: MusicGenerateOptions) -> Result<()> {
    let mut body = json!({
        "model": options.model,
        "prompt": options.prompt,
        "stream": options.stream,
    });

    if let Some(lyrics) = options.lyrics {
        body["lyrics"] = json!(lyrics);
    }
    if let Some(output_format) = options.output_format.clone() {
        body["output_format"] = json!(output_format);
    }
    if let Some(audio_setting) = options.audio_setting_json {
        let parsed: Value = serde_json::from_str(&audio_setting)
            .context("Failed to parse --audio-setting-json as JSON.")?;
        body["audio_setting"] = parsed;
    }

    if options.stream {
        let response = client.post_json_raw("/v1/music_generation", &body).await?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");

        if content_type.contains("application/json") {
            let value: Value = response.json().await?;
            handle_music_response(client, &value, &options.output_dir).await?;
        } else {
            let bytes = response.bytes().await?;
            let extension = options.output_format.unwrap_or_else(|| "wav".to_string());
            let filename = timestamped_filename("music", &extension);
            let path = output_path(&options.output_dir, &filename);
            write_bytes(&path, &bytes)?;
            println!("{} {}", "Saved".green().bold(), path.display());
        }
    } else {
        let response: Value = client.post_json("/v1/music_generation", &body).await?;
        handle_music_response(client, &response, &options.output_dir).await?;
    }

    Ok(())
}

async fn handle_music_response(
    client: &MiniMaxClient,
    response: &Value,
    output_dir: &PathBuf,
) -> Result<()> {
    if let Some(url) = response
        .get("audio_url")
        .or_else(|| response.get("url"))
        .and_then(|value| value.as_str())
    {
        let bytes = client.get_bytes(url).await?;
        let extension = extension_from_url(url).unwrap_or_else(|| "wav".to_string());
        let filename = timestamped_filename("music", &extension);
        let path = output_path(output_dir, &filename);
        write_bytes(&path, &bytes)?;
        println!("{} {}", "Saved".green().bold(), path.display());
        return Ok(());
    }

    if let Some(b64) = response
        .get("audio")
        .or_else(|| response.get("audio_base64"))
        .and_then(|value| value.as_str())
    {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .context("Failed to decode music payload.")?;
        let filename = timestamped_filename("music", "wav");
        let path = output_path(output_dir, &filename);
        write_bytes(&path, &bytes)?;
        println!("{} {}", "Saved".green().bold(), path.display());
        return Ok(());
    }

    println!("{}", "No audio payload found in response.".yellow());
    println!("{}", pretty_json(response));
    Ok(())
}
