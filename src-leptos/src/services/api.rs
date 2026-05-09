use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{DomStringMap, Request, RequestInit, RequestMode, Response};

const BASE: &str = "/api";

/*
 *  @fetch — a wrapper around the browser's fetch API.
 *  @param method — the HTTP method to use.
 *  @param path — the path to fetch, relative to the base URL.
 *  @param body — the body of the request, if any.
 */
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

/*
 *  @ui_fetch — a wrapper around the browser's fetch API, with CORS enabled.
 *  @param method — the HTTP method to use.
 *  @param path — the path to fetch, relative to the base URL.
 *  @param body — the body of the request, if any.
 */
async fn ui_fetch(method: &str, path: &str, body: Option<&str>) -> Result<Response, String> {
    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::SameOrigin);
    if let Some(b) = body {
        opts.set_body(&wasm_bindgen::JsValue::from_str(b));
    }

    let req = Request::new_with_str_and_init(path, &opts).map_err(|e| format!("{e:?}"))?;

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

async fn read_text(resp: Response) -> Result<String, String> {
    let promise = resp.text().map_err(|e| format!("{e:?}"))?;
    let val = JsFuture::from(promise)
        .await
        .map_err(|e| format!("{e:?}"))?;
    val.as_string()
        .ok_or_else(|| "response not a string".to_string())
}

// API Functions

/*
 *  @Auth
 *  POST /ui/session — server validates key and sets httpOnly cookie.
 *  DELETE /ui/session — clears the session cookie.
 */

pub async fn login(key: &str) -> Result<(), String> {
    let body = serde_json::to_string(&super::types::LoginBody { key }).unwrap();
    let resp = ui_fetch("POST", "/ui/session", Some(&body)).await?;
    match resp.status() {
        200 => Ok(()),
        401 => Err("Invalid token".to_string()),
        s => Err(format!("Unexpected status {s}")),
    }
}

pub async fn logout() -> Result<(), String> {
    ui_fetch("DELETE", "/ui/session", None).await?;
    Ok(())
}

pub async fn probe_auth() -> Result<(), String> {
    let resp = fetch("GET", "/functions", None).await?;
    match resp.status() {
        200 => Ok(()),
        401 => Err("Unauthorized".to_string()),
        s => Err(format!("Unexpected status {s}")),
    }
}

/*
 *  @Functions
 *  GET /functions — lists all functions.
 *  GET /functions/{id} — gets a function by id.
 *  DELETE /functions/{id} — deletes a function by id.
 *  POST /functions — creates a new function.
 */

pub async fn list_functions() -> Result<Vec<super::types::FunctionRecord>, String> {
    let resp = fetch("GET", "/functions", None).await?;
    if resp.status() == 401 {
        return Err("Unauthorized".to_string());
    }
    let body = read_text(resp).await?;

    serde_json::from_str(&body).map_err(|e| format!("Parse error: {e:?}"))
}

pub async fn get_function(id: &str) -> Result<super::types::FunctionRecord, String> {
    let resp = fetch("GET", &format!("functions/{id}"), None).await?;
    if resp.status() == 401 {
        return Err("Unauthorized".to_string());
    }
    if resp.status() == 404 {
        return Err("Not found".to_string());
    }
    let body = read_text(resp).await?;
    serde_json::from_str(&body).map_err(|e| format!("Parse error: {e:?}"))
}

pub async fn delete_function(id: &str) -> Result<(), String> {
    let resp = fetch("DELETE", &format!("functions/{id}"), None).await?;
    if resp.status() == 401 {
        return Err("Unauthorized".to_string());
    }
    if resp.status() == 404 {
        return Err("Not found".to_string());
    }
    Ok(())
}

pub async fn deploy_function(req: super::types::FunctionDeployRequest) -> Result<(), String> {
    use web_sys::FormData;
    let form = FormData::new().map_err(|e| format!("{e:?}"))?;
    form.append_with_str("id", &req.id)
        .map_err(|e| format!("{e:?}"))?;

    form.append_with_str("route", &req.route)
        .map_err(|e| format!("{e:?}"))?;

    if let Some(subdomain) = &req.subdomain {
        form.append_with_str("subdomain", subdomain)
            .map_err(|e| format!("{e:?}"))?;
    }

    let uint8 = js_sys::Uint8Array::from(req.wasm_bytes.as_slice());

    let array = js_sys::Array::new();
    array.push(&uint8.buffer());

    let blob = web_sys::Blob::new_with_u8_array_sequence(&array).map_err(|e| format!("{e:?}"))?;

    let id = &req.id;
    form.append_with_blob_and_filename("wasm", &blob, &format!("{id}.wasm"))
        .map_err(|e| format!("{e:?}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&form);
    opts.set_mode(RequestMode::SameOrigin);

    let req = Request::new_with_str_and_init(&format!("{BASE}/functions"), &opts)
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or("no window")?;
    let resp: Response = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|e: wasm_bindgen::JsValue| format!("{e:?}"))?;

    match resp.status() {
        200 | 201 => Ok(()),
        400 => Err(format!("bad request")),
        401 => Err(format!("unauthorized")),
        403 => Err(format!("forbidden")),
        404 => Err(format!("not found")),
        500 => Err(format!("internal server error")),
        s => {
            let msg = read_text(resp).await.unwrap_or_default();
            Err(format!("Deploy failed ({s}): {msg}"))
        }
    }
}
