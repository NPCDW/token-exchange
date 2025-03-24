use serde::{Deserialize, Serialize};
use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();
    router
        .get_async("/hi", |_req, _ctx| async move {
            Response::from_html("<h1>Hello, Cloudflare Worker!</h1>")
        })
        .post_async("/trakt", |req, ctx| async move {
            trakt(req, ctx).await
        })
        .run(req, env).await
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

pub async fn trakt(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body = match req.text().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("body is error: {}", e), 400),
    };
    let body: TraktParam = match serde_json::from_str(&body) {
        Ok(body) => body,
        Err(e) => return Response::error(format!("body is error: {}", e), 400),
    };
    if body.code.is_none() && body.refresh_token.is_none() {
        return Response::error("refresh_token and code cannot both be empty", 400);
    }
    let client_id = ctx.env.var("TRAKT_CLIENT_ID");
    if let Err(_) = client_id {
        return Response::error("TRAKT_CLIENT_ID is error", 503);
    }
    let client_secret = ctx.env.var("TRAKT_CLIENT_SECRET");
    if let Err(_) = client_id {
        return Response::error("TRAKT_CLIENT_SECRET is error", 503);
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
        Err(_) => return Response::error("serde_json to_string error", 500),
    };
    let res = reqwest::Client::new()
        .post("https://api.trakt.tv/oauth/token")
        .header(reqwest::header::CONTENT_TYPE, reqwest::header::HeaderValue::from_static("application/json"))
        .body(body)
        .send()
        .await;
    match res {
        Err(err) => return Response::error(format!("reqwest request error: {}", err), 500),
        Ok(response) => return Ok(Response::builder()
            .with_status(response.status().as_u16())
            .with_headers(response.headers().clone().into())
            .body(ResponseBody::Body(response.bytes().await.unwrap().to_vec()))),
    }
}
