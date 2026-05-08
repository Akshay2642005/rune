use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

const BASE: &str = "/api";

async fn fetch(method: &str, path: &str, body: Option<&str>) -> Result<Response, String> {
    let opts = RequestInit::new();
    opts.set_method(method);
    // SameOrigin ensures the httpOnly cookie is sent automatically
    opts.set_mode(RequestMode::SameOrigin);
    if let Some(b) = body {
        opts.set_body(&wasm_bindgen::JsValue::from_str(b));
    }

    let req = Request::new_with_str_and_init(&format!("{BASE}{path}"), &opts)
        .map_err(|e| format!("{e:?}"))?;

    if body.is_some() {
        req.headers()
            .set("Content-Type", "application/json")
            .map_err(|e| format!("{e:?}"))?;
    }

    let window = web_sys::window().ok_or("no window")?;
    let resp: Response = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e: wasm_bindgen::JsValue| format!("{e:?}"))?;

    Ok(resp)
}

async fn ui_fetch(method: &str, path: &str, body: Option<&str>) -> Result<Response, String> {
    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::SameOrigin);
    if let Some(b) = body {
        opts.set_body(&wasm_bindgen::JsValue::from_str(b));
    }

    let req = Request::new_with_str_and_init(path, &opts)
        .map_err(|e| format!("{e:?}"))?;

    if body.is_some() {
        req.headers()
            .set("Content-Type", "application/json")
            .map_err(|e| format!("{e:?}"))?;
    }

    let window = web_sys::window().ok_or("no window")?;
    let resp: Response = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e: wasm_bindgen::JsValue| format!("{e:?}"))?;

    Ok(resp)
}

#[derive(Serialize)]
struct LoginBody<'a> {
    key: &'a str,
}

/// POST /ui/session — server validates key and sets httpOnly cookie.
pub async fn login(key: &str) -> Result<(), String> {
    let body = serde_json::to_string(&LoginBody { key }).unwrap();
    let resp = ui_fetch("POST", "/ui/session", Some(&body)).await?;
    match resp.status() {
        200 => Ok(()),
        401 => Err("Invalid token".to_string()),
        s => Err(format!("Unexpected status {s}")),
    }
}

/// DELETE /ui/session — clears the session cookie.
pub async fn logout() -> Result<(), String> {
    ui_fetch("DELETE", "/ui/session", None).await?;
    Ok(())
}

/// Probe auth by hitting a protected endpoint — 200 means cookie is valid.
pub async fn probe_auth() -> Result<(), String> {
    let resp = fetch("GET", "/functions", None).await?;
    match resp.status() {
        200 => Ok(()),
        401 => Err("Unauthorized".to_string()),
        s => Err(format!("Unexpected status {s}")),
    }
}
