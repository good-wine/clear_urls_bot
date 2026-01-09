use std::env;
use dotenvy::dotenv;

#[derive(Clone)]
pub struct Config {
    pub bot_token: String,
    pub bot_username: String,
    pub database_url: String,
    pub server_addr: String,
    pub dashboard_url: String,
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
        let clearurls_source = env::var("CLEARURLS_SOURCE").unwrap_or_else(|_| "https://raw.githubusercontent.com/ClearURLs/Rules/refs/heads/master/data.min.json".to_string());

        Self {
            bot_token,
            bot_username,
            database_url,
            server_addr,
            dashboard_url,
            clearurls_source,
        }
    }
}