use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use anyhow::Result;
use crate::models::{UserConfig, ChatConfig};

#[derive(Clone)]
pub struct Db {
    pool: Pool<Sqlite>,
}

impl Db {
    pub async fn new(database_url: &str) -> Result<Self> {
        if database_url.starts_with("sqlite:") {
            let path = database_url.trim_start_matches("sqlite:");
            if !std::path::Path::new(path).exists() {
                std::fs::File::create(path)?;
            }
        }

        let pool = SqlitePoolOptions::new()
            .connect(database_url)
            .await?;

        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_configs (
                user_id INTEGER PRIMARY KEY,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                mode TEXT NOT NULL DEFAULT 'reply',
                ignored_domains TEXT NOT NULL DEFAULT '',
                cleaned_count INTEGER NOT NULL DEFAULT 0
            )"
        )
        .execute(&self.pool)
        .await?;

        // Add columns if they don't exist (migrations)
        let _ = sqlx::query("ALTER TABLE user_configs ADD COLUMN ignored_domains TEXT NOT NULL DEFAULT ''")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE user_configs ADD COLUMN cleaned_count INTEGER NOT NULL DEFAULT 0")
            .execute(&self.pool)
            .await;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_configs (
                chat_id INTEGER PRIMARY KEY,
                title TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                added_by INTEGER NOT NULL
            )"
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_config(&self, user_id: i64) -> Result<UserConfig> {
        let config = sqlx::query_as::<_, UserConfig>(
            "SELECT * FROM user_configs WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(config.unwrap_or(UserConfig {
            user_id,
            enabled: true,
            mode: "reply".to_string(),
            ignored_domains: String::new(),
            cleaned_count: 0,
        }))
    }

    pub async fn save_user_config(&self, config: &UserConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_configs (user_id, enabled, mode, ignored_domains, cleaned_count) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET enabled = ?, mode = ?, ignored_domains = ?, cleaned_count = ?"
        )
        .bind(config.user_id)
        .bind(config.enabled)
        .bind(&config.mode)
        .bind(&config.ignored_domains)
        .bind(config.cleaned_count)
        .bind(config.enabled)
        .bind(&config.mode)
        .bind(&config.ignored_domains)
        .bind(config.cleaned_count)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn increment_cleaned_count(&self, user_id: i64, amount: i64) -> Result<()> {
        sqlx::query(
            "UPDATE user_configs SET cleaned_count = cleaned_count + ? WHERE user_id = ?"
        )
        .bind(amount)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_chat_config(&self, chat_id: i64) -> Result<ChatConfig> {
        let config = sqlx::query_as::<_, ChatConfig>(
            "SELECT * FROM chat_configs WHERE chat_id = ?"
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(config.unwrap_or(ChatConfig {
            chat_id,
            title: None,
            enabled: true,
            added_by: 0,
        }))
    }

    pub async fn save_chat_config(&self, config: &ChatConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO chat_configs (chat_id, title, enabled, added_by) VALUES (?, ?, ?, ?)
             ON CONFLICT(chat_id) DO UPDATE SET title = ?, enabled = ?"
        )
        .bind(config.chat_id)
        .bind(&config.title)
        .bind(config.enabled)
        .bind(config.added_by)
        .bind(&config.title)
        .bind(config.enabled)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_chats_for_user(&self, user_id: i64) -> Result<Vec<ChatConfig>> {
        let chats = sqlx::query_as::<_, ChatConfig>(
            "SELECT * FROM chat_configs WHERE added_by = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(chats)
    }
}