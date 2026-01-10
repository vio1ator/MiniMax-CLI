use base64::Engine;
use crate::client::MiniMaxClient;
use crate::utils::{extension_from_url, output_path, pretty_json, timestamped_filename, write_bytes};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct ImageGenerateOptions {
    pub model: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub aspect_ratio: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub style: Option<String>,
    pub response_format: Option<String>,
    pub seed: Option<u32>,
    pub n: Option<u32>,
    pub prompt_optimizer: Option<bool>,
    pub subject_reference: Vec<String>,
    pub output_dir: PathBuf,
}

pub async fn generate(client: &MiniMaxClient, options: ImageGenerateOptions) -> Result<()> {
    let mut body = json!({
        "model": options.model,
        "prompt": options.prompt,
    });

    if let Some(negative_prompt) = options.negative_prompt {
        body["negative_prompt"] = json!(negative_prompt);
    }
    if let Some(aspect_ratio) = options.aspect_ratio {
        body["aspect_ratio"] = json!(aspect_ratio);
    }
    if let Some(width) = options.width {
        body["width"] = json!(width);
    }
    if let Some(height) = options.height {
        body["height"] = json!(height);
    }
    if let Some(style) = options.style {
        body["style"] = json!(style);
    }
    if let Some(response_format) = options.response_format {
        body["response_format"] = json!(response_format);
    }
    if let Some(seed) = options.seed {
        body["seed"] = json!(seed);
    }
    if let Some(n) = options.n {
        body["n"] = json!(n);
    }
    if let Some(prompt_optimizer) = options.prompt_optimizer {
        body["prompt_optimizer"] = json!(prompt_optimizer);
    }
    if !options.subject_reference.is_empty() {
        body["subject_reference"] = json!(options.subject_reference);
    }

    let response: Value = client.post_json("/v1/image_generation", &body).await?;
    let payloads = extract_image_payloads(&response);

    if payloads.is_empty() {
        println!("{}", "No image payloads found in response.".yellow());
        println!("{}", pretty_json(&response));
        return Ok(());
    }

    let total = payloads.len();
    for (index, payload) in payloads.into_iter().enumerate() {
        let bytes = match payload {
            ImagePayload::Url(url) => {
                let bytes = client.get_bytes(&url).await?;
                (bytes.to_vec(), extension_from_url(&url).unwrap_or_else(|| "png".to_string()))
            }
            ImagePayload::Base64(data) => {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(data.trim())
                    .context("Failed to decode base64 image payload.")?;
                (decoded, "png".to_string())
            }
        };

        let filename = if total == 1 {
            timestamped_filename("image", &bytes.1)
        } else {
            timestamped_filename(&format!("image_{}", index + 1), &bytes.1)
        };
        let path = output_path(&options.output_dir, &filename);
        write_bytes(&path, &bytes.0)?;
        println!("{} {}", "Saved".green().bold(), path.display());
    }

    Ok(())
}

enum ImagePayload {
    Url(String),
    Base64(String),
}

fn extract_image_payloads(response: &Value) -> Vec<ImagePayload> {
    let mut payloads = Vec::new();
    if let Some(data) = response.get("data").and_then(|value| value.as_array()) {
        for item in data {
            if let Some(url) = item
                .get("url")
                .or_else(|| item.get("image_url"))
                .and_then(|value| value.as_str())
            {
                payloads.push(ImagePayload::Url(url.to_string()));
            } else if let Some(b64) = item
                .get("b64_json")
                .or_else(|| item.get("image_base64"))
                .or_else(|| item.get("image"))
                .and_then(|value| value.as_str())
            {
                payloads.push(ImagePayload::Base64(b64.to_string()));
            }
        }
    }

    if payloads.is_empty() {
        if let Some(url) = response
            .get("image_url")
            .or_else(|| response.get("url"))
            .and_then(|value| value.as_str())
        {
            payloads.push(ImagePayload::Url(url.to_string()));
        } else if let Some(b64) = response
            .get("image_base64")
            .or_else(|| response.get("b64_json"))
            .and_then(|value| value.as_str())
        {
            payloads.push(ImagePayload::Base64(b64.to_string()));
        }
    }

    payloads
}
