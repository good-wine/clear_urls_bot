use axum::{
    extract::{State, Query, FromRef},
    response::{Html, Redirect, IntoResponse, Response},
    routing::{get, post},
    Router,
    Form,
    http::{HeaderValue, header},
};
use axum_extra::extract::cookie::{Cookie, SignedCookieJar, Key};
use crate::{db::Db, config::Config, models::{UserConfig, ChatConfig}};
use askama::Template;
use std::collections::HashMap;
use hmac::{Hmac, Mac};
use tracing::info;
use hex;
use tower_http::set_header::SetResponseHeaderLayer;
use time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Config,
    pub key: Key,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    bot_username: String,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    user: TelegramUserSession,
    config: UserConfig,
    chats: Vec<ChatConfig>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
struct TelegramUserSession {
    id: i64,
    first_name: String,
    username: Option<String>,
    photo_url: Option<String>,
}

pub async fn run_server(config: Config, db: Db) {
    let key = Key::generate();
    
    let state = AppState {
        db,
        config: config.clone(),
        key,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/login", get(login_page))
        .route("/favicon.ico", get(|| async { axum::http::StatusCode::NO_CONTENT }))
        .route("/auth/telegram/callback", get(auth_callback))
        .route("/logout", get(logout))
        .route("/dashboard/update", post(update_config))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; script-src 'self' https://telegram.org; frame-src https://oauth.telegram.org https://telegram.org; style-src 'self' 'unsafe-inline'; img-src 'self' https://t.me https://telegram.org data:; connect-src 'self';"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("SAMEORIGIN"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.server_addr).await.unwrap();
    info!("Web dashboard listening on {}", config.server_addr);
    axum::serve(listener, app).await.unwrap();
}

async fn index(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Response {
    if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            let user_config = state.db.get_user_config(user.id).await.unwrap_or_default();

            let chats = state.db.get_chats_for_user(user.id).await.unwrap_or_default();

            let template = DashboardTemplate {
                user,
                config: user_config,
                chats,
            };
            return Html(template.render().unwrap()).into_response();
        }
    }
    Redirect::to("/login").into_response()
}

async fn login_page(State(state): State<AppState>) -> Html<String> {
    let template = LoginTemplate {
        bot_username: state.config.bot_username,
    };
    Html(template.render().unwrap())
}

async fn auth_callback(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = &state.config.bot_token;
    
    if verify_telegram_auth(&params, token) {
        let user = TelegramUserSession {
            id: params.get("id").unwrap().parse().unwrap_or(0),
            first_name: params.get("first_name").cloned().unwrap_or_default(),
            username: params.get("username").cloned(),
            photo_url: params.get("photo_url").cloned(),
        };

        let cookie_val = serde_json::to_string(&user).unwrap();
        let cookie = Cookie::build(("user_session", cookie_val))
            .path("/")
            .http_only(true)
            .max_age(Duration::days(30))
            .same_site(axum_extra::extract::cookie::SameSite::Lax)
            .build();

        return (jar.add(cookie), Redirect::to("/")).into_response();
    }

    (jar, Redirect::to("/login")).into_response()
}

async fn logout(_state: State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    (jar.remove(Cookie::from("user_session")), Redirect::to("/login"))
}

#[derive(serde::Deserialize)]
struct UpdateForm {
    enabled: Option<String>,
    mode: String,
    ignored_domains: String,
}

async fn update_config(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<UpdateForm>,
) -> impl IntoResponse {
    if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            let user_config = state.db.get_user_config(user.id).await.unwrap_or_default();
            let enabled = form.enabled.is_some();
            let config = UserConfig {
                user_id: user.id,
                enabled,
                mode: form.mode,
                ignored_domains: form.ignored_domains,
                cleaned_count: user_config.cleaned_count,
            };
            let _ = state.db.save_user_config(&config).await;
        }
    }
    Redirect::to("/")
}

fn verify_telegram_auth(params: &HashMap<String, String>, token: &str) -> bool {
    let hash = match params.get("hash") {
        Some(h) => h,
        None => return false,
    };

    // Verify auth_date to prevent replay attacks (max 24h old)
    if let Some(auth_date_str) = params.get("auth_date") {
        if let Ok(auth_date) = auth_date_str.parse::<u64>() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now - auth_date > 86400 {
                return false;
            }
        }
    }

    let mut keys: Vec<&String> = params.keys().filter(|k| k.as_str() != "hash").collect();
    keys.sort();

    let data_check_string = keys.iter() 
        .map(|k| format!("{}={}", k, params.get(*k).unwrap()))
        .collect::<Vec<String>>()
        .join("\n");

    use sha2::{Digest, Sha256};
    let secret_key = Sha256::digest(token.as_bytes());
    
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC can take key of any size");
    mac.update(data_check_string.as_bytes());
    
    let computed_hash = hex::encode(mac.finalize().into_bytes());
    
    computed_hash == *hash
}