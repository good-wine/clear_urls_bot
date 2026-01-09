use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, Default)]
pub struct UserConfig {
    pub user_id: i64,
    pub enabled: bool,
    pub ai_enabled: bool,
    pub mode: String, // "reply" or "delete"
    pub ignored_domains: String, // Comma-separated list
    pub cleaned_count: i64,
    pub language: String, // "en", "it", etc.
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct ChatConfig {
    pub chat_id: i64,
    pub title: Option<String>,
    pub enabled: bool,
    pub added_by: i64,
    pub mode: String, // "reply", "delete", or "default"
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