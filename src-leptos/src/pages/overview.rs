use crate::{components::shell::Shell, services::api::list_functions};
use leptos::prelude::*;

#[component]
pub fn OverviewPage() -> impl IntoView {
    let functions = LocalResource::new(|| async { list_functions().await });
    view! {
        <Shell title="Overview">
            <p class="text-on-surface-variant text-sm">"Overview coming soon."</p>
        </Shell>
    }
}
