use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tower_service::Service;
use worker::*;

fn router(_env: Env) -> Router {
    Router::new().route("/trakt", post(trakt)).with_state(AxumState {
        env_wrapper: _env,
    })
}

#[derive(Clone)]
pub struct AxumState {
    pub env_wrapper: Env,
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    console_error_panic_hook::set_once();
    Ok(router(_env).call(req).await?)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraktParam {
    pub code: Option<String>,
    pub refresh_token: Option<String>,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraktExchangeTokenParam {
    pub code: Option<String>,
    pub refresh_token: Option<String>,
    pub redirect_uri: String,
    pub client_id: String,
    pub client_secret: String,
    pub grant_type: String,
}

pub async fn trakt(State(state): State<AxumState>, body: Json<TraktParam>) -> axum::response::Response {
    if body.code.is_none() && body.refresh_token.is_none() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::http::HeaderMap::new(),
            axum::body::Body::new("refresh_token and code cannot both be empty".to_string())
        ).into_response();
    }
    let client_id = state.env_wrapper.var("TRAKT_CLIENT_ID");
    if let Err(_) = client_id {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::http::HeaderMap::new(),
            axum::body::Body::new("TRAKT_CLIENT_ID is error".to_string())
        ).into_response();
    }
    let client_secret = state.env_wrapper.var("TRAKT_CLIENT_SECRET");
    if let Err(_) = client_id {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::http::HeaderMap::new(),
            axum::body::Body::new("TRAKT_CLIENT_SECRET is error".to_string())
        ).into_response();
    }

    let grant_type = match &body.code {
        Some(_) => "authorization_code",
        None => "refresh_token",
    };
    let body = match serde_json::to_string(&TraktExchangeTokenParam {
        code: body.code.clone(),
        refresh_token: body.refresh_token.clone(),
        redirect_uri: body.redirect_uri.clone(),
        client_id: client_id.unwrap().to_string(),
        client_secret: client_secret.unwrap().to_string(),
        grant_type: grant_type.to_string(),
    }) {
        Ok(body) => body,
        Err(_) => return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::http::HeaderMap::new(),
            axum::body::Body::new("serde_json to_string error".to_string())
        ).into_response(),
    };
    let res = reqwest::Client::new()
        .post("https://api.trakt.tv/oauth/token")
        .header(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("application/json"))
        .body(body)
        .send()
        .await;
    if let Err(err) = res {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::http::HeaderMap::new(),
            axum::body::Body::new(err.to_string())
        ).into_response()
    }
    let response = res.unwrap();
    return (
        response.status(),
        response.headers().clone(),
        axum::body::Body::from_stream(response.bytes_stream())
    ).into_response();
}
