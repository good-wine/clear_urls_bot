use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct UserConfig {
    pub user_id: i64,
    pub enabled: i32,
    pub ai_enabled: i32,
    pub mode: String, // "reply" or "delete"
    pub ignored_domains: String, // Comma-separated list
    pub cleaned_count: i64,
    pub language: String, // "en", "it", etc.
}

impl UserConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }
    pub fn is_ai_enabled(&self) -> bool {
        self.ai_enabled != 0
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            user_id: 0,
            enabled: 1,
            ai_enabled: 0,
            mode: "reply".to_string(),
            ignored_domains: String::new(),
            cleaned_count: 0,
            language: "en".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct ChatConfig {
    pub chat_id: i64,
    pub title: Option<String>,
    pub enabled: i32,
    pub added_by: i64,
    pub mode: String, // "reply", "delete", or "default"
}

impl ChatConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            chat_id: 0,
            title: None,
            enabled: 1,
            added_by: 0,
            mode: "default".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct CustomRule {
    pub id: i64,
    pub user_id: i64,
    pub pattern: String, // Regex or string to match in query params
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct CleanedLink {
    pub id: i64,
    pub user_id: i64,
    pub original_url: String,
    pub cleaned_url: String,
    pub provider_name: Option<String>,
    pub timestamp: i64,
}