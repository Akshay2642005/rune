use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct SearchContext {
    pub query: RwSignal<String>,
}

impl SearchContext {
    pub fn provide() {
        provide_context(SearchContext { query: RwSignal::new(String::new()) });
    }
    pub fn get() -> Self {
        use_context::<SearchContext>().expect("SearchContext not provided")
    }
}
