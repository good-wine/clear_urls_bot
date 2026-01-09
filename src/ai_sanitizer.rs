use reqwest::Client;
use serde_json::{json, Value};
use anyhow::{Result, anyhow};
use tracing::debug;
use crate::config::Config;

#[derive(Clone)]
pub struct AiEngine {
    client: Client,
    api_key: Option<String>,
    api_base: String,
    model: String,
}

impl AiEngine {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            api_key: config.ai_api_key.clone(),
            api_base: config.ai_api_base.clone(),
            model: config.ai_model.clone(),
        }
    }

    pub async fn sanitize(&self, url: &str) -> Result<Option<String>> {
        let api_key = match &self.api_key {
            Some(key) => key,
            None => return Ok(None),
        };

        debug!("Requesting AI sanitization for: {}", url);

        let prompt = format!(
            "You are a URL sanitizer. Remove all tracking parameters from the following URL. \n            Tracking parameters are things like utm_source, fbclid, gclid, etc., but also provider-specific ones. \n            Return ONLY the cleaned URL and nothing else. If the URL is already clean or no tracking is found, return the same URL. \n            URL: {}",
            url
        );

        let response = self.client.post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": "You are a specialized tool for cleaning URLs from tracking parameters. Output only the cleaned URL."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.0
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let err = response.text().await?;
            return Err(anyhow!("AI API error: {}", err));
        }

        let data: Value = response.json().await?;
        let cleaned = data["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.trim().to_string());

        if let Some(cleaned_url) = cleaned {
            if cleaned_url != url {
                debug!("AI cleaned URL: {} -> {}", url, cleaned_url);
                return Ok(Some(cleaned_url));
            }
        }

        Ok(None)
    }
}
