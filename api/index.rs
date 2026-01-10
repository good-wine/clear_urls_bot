use clear_urls_bot::{
    config::Config,
    db::Db,
    web::{create_app, AppState},
};
use vercel_runtime::{run, Request, Response, ResponseBody, Error, AppState as VercelState};
use axum_extra::extract::cookie::Key;
use tower::{ServiceExt, service_fn};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(|(state, req)| handler(state, req))).await
}

pub async fn handler(_state: VercelState, req: Request) -> Result<Response<ResponseBody>, Error> {
    let config = Config::from_env();
    // In serverless, we must use a remote database (Supabase/Postgres)
    let db = Db::new(&config.database_url).await.map_err(|e| Error::from(e.to_string()))?;
    
    // Serverless functions are short-lived, so broadcast channels are local to the invocation
    let (event_tx, _) = tokio::sync::broadcast::channel::<serde_json::Value>(100);

    let key = if let Some(ref k) = config.cookie_key {
        Key::from(k.as_bytes())
    } else {
        Key::generate()
    };

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        key,
        event_tx,
    };

    let app = create_app(state);
    
    let (parts, incoming) = req.into_parts();
    let axum_req = axum::http::Request::from_parts(parts, axum::body::Body::new(incoming));
    
    let axum_response = app.oneshot(axum_req).await.map_err(|e| Error::from(e.to_string()))?;
    
    let (parts, body) = axum_response.into_parts();
    let bytes = axum::body::to_bytes(body, 10 * 1024 * 1024)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    
    Ok(Response::from_parts(parts, ResponseBody::from(bytes.to_vec())))
}
