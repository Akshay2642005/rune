use leptos::prelude::*;
use leptos_router::hooks::use_location;
use time::Date;

/// URL query parameter keys
#[allow(non_camel_case_types)]
pub struct QUERY;

impl QUERY {
    pub const PAGE: &'static str = "page";
    pub const START_DATE: &'static str = "start_date";
    pub const END_DATE: &'static str = "end_date";
}

pub struct QueryUtils;

impl QueryUtils {
    /// Extract a query parameter value as a reactive closure (call it like a function)
    pub fn extract(key: String) -> impl Fn() -> Option<String> + Clone {
        let location = use_location();
        move || location.query.with(|q| q.get(&key))
    }

    /// Update the URL with new date range parameters using browser history API
    pub fn update_dates_url(start: Option<Date>, end: Option<Date>) {
        let window = leptos::prelude::window();
        let location = window.location();
        let pathname = location.pathname().unwrap_or_default();
        let search = location.search().unwrap_or_default();

        // Parse existing params
        let existing = if search.starts_with('?') { &search[1..] } else { &search };
        let mut params: Vec<String> = existing
            .split('&')
            .filter(|s| !s.is_empty())
            .filter(|s| {
                !s.starts_with(&format!("{}=", QUERY::START_DATE))
                    && !s.starts_with(&format!("{}=", QUERY::END_DATE))
            })
            .map(|s| s.to_string())
            .collect();

        if let Some(s) = start {
            params.push(format!("{}={}", QUERY::START_DATE, s));
        }
        if let Some(e) = end {
            params.push(format!("{}={}", QUERY::END_DATE, e));
        }

        let new_url = if params.is_empty() {
            pathname
        } else {
            format!("{}?{}", pathname, params.join("&"))
        };

        if let Some(history) = window.history().ok() {
            let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&new_url));
        }
    }
}
