use crate::components::shell::Shell;
use leptos::prelude::*;

#[component]
pub fn WorkersPage() -> impl IntoView {
    view! {
        <Shell title="Workers">
            <p class="text-on-surface-variant text-sm">"Workers coming soon."</p>
        </Shell>
    }
}

#[component]
pub fn KeysPage() -> impl IntoView {
    view! {
        <Shell title="API Keys">
            <p class="text-on-surface-variant text-sm">"API Keys coming soon."</p>
        </Shell>
    }
}
