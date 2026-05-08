use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::utils::query::{QUERY, QueryUtils};

const FIRST_PAGE: u32 = 1;

#[derive(Clone)]
pub struct PaginationContext {
    pub current_page: Memo<u32>,
    pub page_href: Callback<u32, String>,
    pub prev_href: Signal<String>,
    pub next_href: Signal<String>,
    pub is_first_page: Signal<bool>,
    pub aria_current: Callback<u32, &'static str>,
}

pub fn use_pagination() -> PaginationContext {
    let location = use_location();
    let current_page_str = QueryUtils::extract(QUERY::PAGE.to_string());

    let current_page = Memo::new(move |_| current_page_str().and_then(|s| s.parse::<u32>().ok()).unwrap_or(FIRST_PAGE));

    let page_href = Callback::new(move |page: u32| {
        location.query.with(|q| {
            // Rebuild query string excluding PAGE param, then add new page
            let search = leptos::prelude::window().location().search().unwrap_or_default();
            let existing = if search.starts_with('?') { &search[1..] } else { &search };
            let mut params: Vec<String> = existing
                .split('&')
                .filter(|s| !s.is_empty() && !s.starts_with(&format!("{}=", QUERY::PAGE)))
                .map(|s| s.to_string())
                .collect();
            params.push(format!("{}={}", QUERY::PAGE, page));
            format!("?{}", params.join("&"))
        })
    });

    let prev_href = Signal::derive(move || {
        let current = current_page.get();
        if current > FIRST_PAGE { page_href.run(current - 1) } else { "#".to_string() }
    });

    let next_href = Signal::derive(move || {
        let current = current_page.get();
        page_href.run(current + 1)
    });

    let is_first_page = Signal::derive(move || current_page.get() <= FIRST_PAGE);

    let aria_current = Callback::new(move |page: u32| if current_page.get() == page { QUERY::PAGE } else { "" });

    PaginationContext { current_page, page_href, prev_href, next_href, is_first_page, aria_current }
}