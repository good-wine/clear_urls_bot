use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use anyhow::Result;
use crate::models::{UserConfig, ChatConfig};

#[derive(Clone)]
pub struct Db {
    pub pool: Pool<Postgres>,
}

impl Db {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_configs (
                user_id BIGINT PRIMARY KEY,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                ai_enabled BOOLEAN NOT NULL DEFAULT FALSE,
                mode TEXT NOT NULL DEFAULT 'reply',
                ignored_domains TEXT NOT NULL DEFAULT '',
                cleaned_count BIGINT NOT NULL DEFAULT 0,
                language TEXT NOT NULL DEFAULT 'en'
            )"
        )
        .execute(&self.pool)
        .await?;

        // Simple check to add columns if they don't exist
        let _ = sqlx::query("ALTER TABLE user_configs ADD COLUMN IF NOT EXISTS ai_enabled BOOLEAN NOT NULL DEFAULT FALSE")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE user_configs ADD COLUMN IF NOT EXISTS ignored_domains TEXT NOT NULL DEFAULT ''")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE user_configs ADD COLUMN IF NOT EXISTS cleaned_count BIGINT NOT NULL DEFAULT 0")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("ALTER TABLE user_configs ADD COLUMN IF NOT EXISTS language TEXT NOT NULL DEFAULT 'en'")
            .execute(&self.pool)
            .await;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_configs (
                chat_id BIGINT PRIMARY KEY,
                title TEXT,
                enabled BOOLEAN NOT NULL DEFAULT TRUE,
                added_by BIGINT NOT NULL,
                mode TEXT NOT NULL DEFAULT 'default'
            )"
        )
        .execute(&self.pool)
        .await?;

        let _ = sqlx::query("ALTER TABLE chat_configs ADD COLUMN IF NOT EXISTS mode TEXT NOT NULL DEFAULT 'default'")
            .execute(&self.pool)
            .await;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS custom_rules (
                id SERIAL PRIMARY KEY,
                user_id BIGINT NOT NULL,
                pattern TEXT NOT NULL
            )"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS cleaned_links (
                id SERIAL PRIMARY KEY,
                user_id BIGINT NOT NULL,
                original_url TEXT NOT NULL,
                cleaned_url TEXT NOT NULL,
                provider_name TEXT,
                timestamp BIGINT NOT NULL
            )"
        )
        .execute(&self.pool)
        .await?;

        let _ = sqlx::query("ALTER TABLE cleaned_links ADD COLUMN IF NOT EXISTS provider_name TEXT")
            .execute(&self.pool)
            .await;

        Ok(())
    }

    pub async fn log_cleaned_link(&self, user_id: i64, original: &str, cleaned: &str, provider: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        sqlx::query(
            "INSERT INTO cleaned_links (user_id, original_url, cleaned_url, provider_name, timestamp) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user_id)
        .bind(original)
        .bind(cleaned)
        .bind(provider)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_history(&self, user_id: i64, limit: i64) -> Result<Vec<crate::models::CleanedLink>> {
        let history = sqlx::query_as::<_, crate::models::CleanedLink>(
            "SELECT * FROM cleaned_links WHERE user_id = $1 ORDER BY timestamp DESC LIMIT $2"
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(history)
    }

    pub async fn get_global_stats(&self) -> Result<(i64, i64)> {
        let total_cleaned: (Option<i64>,) = sqlx::query_as("SELECT SUM(cleaned_count) FROM user_configs")
            .fetch_one(&self.pool)
            .await?;
        let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM user_configs")
            .fetch_one(&self.pool)
            .await?;
        Ok((total_cleaned.0.unwrap_or(0), total_users.0))
    }

    pub async fn get_user_config(&self, user_id: i64) -> Result<UserConfig> {
        let config = sqlx::query_as::<_, UserConfig>(
            "SELECT * FROM user_configs WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(config.unwrap_or(UserConfig {
            user_id,
            enabled: true,
            ai_enabled: false,
            mode: "reply".to_string(),
            ignored_domains: String::new(),
            cleaned_count: 0,
            language: "en".to_string(),
        }))
    }

    pub async fn save_user_config(&self, config: &UserConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_configs (user_id, enabled, ai_enabled, mode, ignored_domains, cleaned_count, language) VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT(user_id) DO UPDATE SET enabled = $2, ai_enabled = $3, mode = $4, ignored_domains = $5, cleaned_count = $6, language = $7"
        )
        .bind(config.user_id)
        .bind(config.enabled)
        .bind(config.ai_enabled)
        .bind(&config.mode)
        .bind(&config.ignored_domains)
        .bind(config.cleaned_count)
        .bind(&config.language)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn increment_cleaned_count(&self, user_id: i64, amount: i64) -> Result<()> {
        sqlx::query(
            "UPDATE user_configs SET cleaned_count = cleaned_count + $1 WHERE user_id = $2"
        )
        .bind(amount)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_custom_rules(&self, user_id: i64) -> Result<Vec<crate::models::CustomRule>> {
        let rules = sqlx::query_as::<_, crate::models::CustomRule>(
            "SELECT * FROM custom_rules WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rules)
    }

    pub async fn add_custom_rule(&self, user_id: i64, pattern: &str) -> Result<()> {
        sqlx::query("INSERT INTO custom_rules (user_id, pattern) VALUES ($1, $2)")
            .bind(user_id)
            .bind(pattern)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_stats_by_day(&self, user_id: i64) -> Result<Vec<(String, i64)>> {
        let stats = sqlx::query_as::<_, (String, i64)>(
            "SELECT to_char(to_timestamp(timestamp), 'YYYY-MM-DD') as day, COUNT(*) 
             FROM cleaned_links 
             WHERE user_id = $1 
             GROUP BY day ORDER BY day DESC LIMIT 7"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(stats)
    }

    pub async fn get_chat_config(&self, chat_id: i64) -> Result<ChatConfig> {
        let config = sqlx::query_as::<_, ChatConfig>(
            "SELECT * FROM chat_configs WHERE chat_id = $1"
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(config.unwrap_or(ChatConfig {
            chat_id,
            title: None,
            enabled: true,
            added_by: 0,
            mode: "default".to_string(),
        }))
    }

    pub async fn save_chat_config(&self, config: &ChatConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO chat_configs (chat_id, title, enabled, added_by, mode) VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT(chat_id) DO UPDATE SET title = $2, enabled = $3, mode = $5"
        )
        .bind(config.chat_id)
        .bind(&config.title)
        .bind(config.enabled)
        .bind(config.added_by)
        .bind(&config.mode)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_chats_for_user(&self, user_id: i64) -> Result<Vec<ChatConfig>> {
        let chats = sqlx::query_as::<_, ChatConfig>(
            "SELECT * FROM chat_configs WHERE added_by = $1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(chats)
    }
}