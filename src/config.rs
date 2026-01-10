use std::env;
use dotenvy::dotenv;
use url::Url;

#[derive(Clone)]
pub struct Config {
    pub bot_token: String,
    pub bot_username: String,
    pub database_url: String,
    pub server_addr: String,
    pub dashboard_url: Url,
    pub admin_id: i64,
    pub clearurls_source: String,
    pub cookie_key: Option<String>,
    pub ai_api_key: Option<String>,
    pub ai_api_base: String,
    pub ai_model: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();

        let bot_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN must be set");
        let bot_username = env::var("BOT_USERNAME").expect("BOT_USERNAME must be set");
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:bot.db".to_string());
        let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| format!("0.0.0.0:{}", port));
        let dashboard_url_str = env::var("DASHBOARD_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
        let dashboard_url = Url::parse(&dashboard_url_str).expect("DASHBOARD_URL must be a valid URL");
        let admin_id = env::var("ADMIN_ID").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);
        let clearurls_source = env::var("CLEARURLS_SOURCE").unwrap_or_else(|_| "https://raw.githubusercontent.com/ClearURLs/Rules/refs/heads/master/data.min.json".to_string());
        let cookie_key = env::var("COOKIE_KEY").ok();
        let ai_api_key = env::var("AI_API_KEY").ok();
        let ai_api_base = env::var("AI_API_BASE").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let ai_model = env::var("AI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());

        Self {
            bot_token,
            bot_username,
            database_url,
            server_addr,
            dashboard_url,
            admin_id,
            clearurls_source,
            cookie_key,
            ai_api_key,
            ai_api_base,
            ai_model,
        }
    }

    pub fn validate(&self) {
        if self.bot_token.is_empty() || !self.bot_token.contains(':') {
            panic!("FATAL: TELOXIDE_TOKEN non è valido o è vuoto. Controlla il file .env");
        }
        if self.bot_username.is_empty() {
            panic!("FATAL: BOT_USERNAME deve essere configurato");
        }

        // Render Reserved Ports check
        let reserved_ports = ["18012", "18013", "19099"];
        for port in reserved_ports {
            if self.server_addr.contains(port) {
                panic!("FATAL: Port {} is reserved by Render and cannot be used.", port);
            }
        }
    }
}