use axum::{
    extract::{State, Query, FromRef},
    response::{Html, Redirect, IntoResponse, Response, sse::{Event, Sse}},
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
use futures::stream::Stream;
use std::convert::Infallible;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Config,
    pub key: Key,
    pub event_tx: tokio::sync::broadcast::Sender<serde_json::Value>,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TelegramUserSession {
    pub id: i64,
    pub first_name: String,
    pub username: Option<String>,
    pub photo_url: Option<String>,
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
    history: Vec<crate::models::CleanedLink>,
    custom_rules: Vec<crate::models::CustomRule>,
    stats_by_day: Vec<(String, i64)>,
    admin_id: i64,
    tr: crate::i18n::Translations,
}

#[derive(serde::Deserialize)]
struct CustomRuleForm {
    pattern: String,
}

pub async fn run_server(config: Config, db: Db, event_tx: tokio::sync::broadcast::Sender<serde_json::Value>) {
    let key = Key::generate();
    
    let state = AppState {
        db,
        config: config.clone(),
        key,
        event_tx,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(|| async { "OK" }))
        .route("/events", get(events_handler))
        .route("/login", get(login_page))
        .route("/favicon.ico", get(|| async { axum::http::StatusCode::NO_CONTENT }))
        .route("/auth/telegram/callback", get(auth_callback))
        .route("/logout", get(logout))
        .route("/dashboard/update", post(update_config))
        .route("/dashboard/chat/toggle/:chat_id", post(toggle_chat))
        .route("/dashboard/chat/mode/:chat_id", post(update_chat_mode))
        .route("/dashboard/custom_rule/add", post(add_custom_rule))
        .route("/dashboard/custom_rule/delete/:id", post(delete_custom_rule))
        .route("/dashboard/export", get(export_history))
        .route("/admin", get(admin_dashboard))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self' https://cdn.jsdelivr.net; script-src 'self' 'unsafe-inline' https://telegram.org https://cdn.jsdelivr.net; frame-src https://oauth.telegram.org https://telegram.org; style-src 'self' 'unsafe-inline'; img-src 'self' https://t.me https://telegram.org data:; connect-src 'self';"),
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
            let history = state.db.get_history(user.id, 10).await.unwrap_or_default();
            let custom_rules = state.db.get_custom_rules(user.id).await.unwrap_or_default();
            let mut stats_by_day = state.db.get_stats_by_day(user.id).await.unwrap_or_default();
            stats_by_day.reverse();

            let tr = crate::i18n::get_translations(&user_config.language);

            let template = DashboardTemplate {
                user,
                config: user_config,
                chats,
                history,
                custom_rules,
                stats_by_day,
                admin_id: state.config.admin_id,
                tr,
            };
            return Html(template.render().unwrap()).into_response();
        }
    }
    Redirect::to("/login").into_response()
}

#[derive(serde::Deserialize)]
struct ChatModeForm {
    mode: String,
}

async fn update_chat_mode(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    axum::extract::Path(chat_id): axum::extract::Path<i64>,
    Form(form): Form<ChatModeForm>,
) -> impl IntoResponse {
    if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            if let Ok(mut chat_config) = state.db.get_chat_config(chat_id).await {
                if chat_config.added_by == user.id {
                    chat_config.mode = form.mode;
                    let _ = state.db.save_chat_config(&chat_config).await;
                }
            }
        }
    }
    Redirect::to("/")
}

async fn add_custom_rule(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<CustomRuleForm>,
) -> impl IntoResponse {
    if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            let _ = state.db.add_custom_rule(user.id, &form.pattern).await;
        }
    }
    Redirect::to("/")
}

async fn delete_custom_rule(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            let _ = sqlx::query("DELETE FROM custom_rules WHERE id = ? AND user_id = ?")
                .bind(id)
                .bind(user.id)
                .execute(&state.db.pool)
                .await;
        }
    }
    Redirect::to("/")
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
    language: String,
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
                language: form.language,
            };
            let _ = state.db.save_user_config(&config).await;
        }
    }
    Redirect::to("/")
}

async fn toggle_chat(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    axum::extract::Path(chat_id): axum::extract::Path<i64>,
) -> impl IntoResponse {
    if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            if let Ok(mut chat_config) = state.db.get_chat_config(chat_id).await {
                if chat_config.added_by == user.id {
                    chat_config.enabled = !chat_config.enabled;
                    let _ = state.db.save_chat_config(&chat_config).await;
                }
            }
        }
    }
    Redirect::to("/")
}

#[derive(Template)]
#[template(path = "admin.html")]
struct AdminTemplate {
    total_cleaned: i64,
    total_users: i64,
}

async fn admin_dashboard(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Response {
    if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            if user.id == state.config.admin_id {
                let (total_cleaned, total_users) = state.db.get_global_stats().await.unwrap_or((0, 0));
                let template = AdminTemplate { total_cleaned, total_users };
                return Html(template.render().unwrap()).into_response();
            }
        }
    }
    Redirect::to("/").into_response()
}

async fn export_history(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Response {
    if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            let history = state.db.get_history(user.id, 1000).await.unwrap_or_default();
            let mut csv = String::from("ID,Original URL,Cleaned URL,Provider,Timestamp\n");
            for link in history {
                csv.push_str(&format!(
                    "{},\"{}\",\"{}\",\"{}\",{}\n",
                    link.id,
                    link.original_url.replace("\"", "\"\""),
                    link.cleaned_url.replace("\"", "\"\""),
                    link.provider_name.unwrap_or_default(),
                    link.timestamp
                ));
            }
            
            return Response::builder()
                .header(header::CONTENT_TYPE, "text/csv")
                .header(header::CONTENT_DISPOSITION, "attachment; filename=\"history.csv\"")
                .body(axum::body::Body::from(csv))
                .unwrap();
        }
    }
    Redirect::to("/login").into_response()
}

async fn events_handler(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let user_id = if let Some(user_cookie) = jar.get("user_session") {
        if let Ok(user) = serde_json::from_str::<TelegramUserSession>(user_cookie.value()) {
            user.id
        } else { 0 }
    } else { 0 };

    let mut rx = state.event_tx.subscribe();

    let stream = async_stream::stream! {
        while let Ok(msg) = rx.recv().await {
            if let Some(target_user_id) = msg.get("user_id").and_then(|id| id.as_i64()) {
                if target_user_id == user_id {
                    yield Ok(Event::default().json_data(msg).unwrap());
                }
            }
        }
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}

fn verify_telegram_auth(params: &HashMap<String, String>, token: &str) -> bool {
    let hash = match params.get("hash") {
        Some(h) => h,
        None => return false,
    };

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
    let mut mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC error");
    mac.update(data_check_string.as_bytes());
    
    let computed_hash = hex::encode(mac.finalize().into_bytes());
    computed_hash == *hash
}
