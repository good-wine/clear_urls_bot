use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct UserConfig {
    pub user_id: i64,
    pub enabled: bool,
    pub mode: String, // "reply" or "delete"
    pub ignored_domains: String, // Comma-separated list
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct ChatConfig {
    pub chat_id: i64,
    pub title: Option<String>,
    pub enabled: bool,
    pub added_by: i64,
}
