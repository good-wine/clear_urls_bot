use std::env;
use dotenvy::dotenv;

#[derive(Clone)]
pub struct Config {
    pub bot_token: String,
    pub bot_username: String,
    pub database_url: String,
    pub server_addr: String,
    pub dashboard_url: String,
    pub admin_id: i64,
    pub clearurls_source: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv().ok();

        let bot_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN must be set");
        let bot_username = env::var("BOT_USERNAME").expect("BOT_USERNAME must be set");
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:bot.db".to_string());
        let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
        let dashboard_url = env::var("DASHBOARD_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
        let admin_id = env::var("ADMIN_ID").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);
        let clearurls_source = env::var("CLEARURLS_SOURCE").unwrap_or_else(|_| "https://raw.githubusercontent.com/ClearURLs/Rules/refs/heads/master/data.min.json".to_string());

        Self {
            bot_token,
            bot_username,
            database_url,
            server_addr,
            dashboard_url,
            admin_id,
            clearurls_source,
        }
    }

    pub fn validate(&self) {
        if self.bot_token.is_empty() || !self.bot_token.contains(':') {
            panic!("FATAL: TELOXIDE_TOKEN non è valido o è vuoto. Controlla il file .env");
        }
        if self.bot_username.is_empty() {
            panic!("FATAL: BOT_USERNAME deve essere configurato");
        }
        if !self.dashboard_url.starts_with("http") {
            panic!("FATAL: DASHBOARD_URL deve essere un URL valido (es. http://localhost:3000)");
        }
    }
}