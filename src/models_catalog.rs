use serde_json::json;

pub struct ModelCategory {
    pub name: &'static str,
    pub models: &'static [&'static str],
}

pub fn categories() -> Vec<ModelCategory> {
    vec![
        ModelCategory {
            name: "text",
            models: &["MiniMax-M2.1", "M2.1-Lightning", "MiniMax-Text-01"],
        },
        ModelCategory {
            name: "image",
            models: &["image-01"],
        },
        ModelCategory {
            name: "video",
            models: &["video-01"],
        },
        ModelCategory {
            name: "audio",
            models: &["speech-01", "speech-02"],
        },
        ModelCategory {
            name: "music",
            models: &["music-01"],
        },
    ]
}

pub fn as_json() -> serde_json::Value {
    let payload = categories()
        .into_iter()
        .map(|category| {
            json!({
                "category": category.name,
                "models": category.models,
            })
        })
        .collect::<Vec<_>>();
    json!(payload)
}
