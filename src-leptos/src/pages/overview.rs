use crate::components::shell::Shell;
use leptos::prelude::*;

#[component]
pub fn OverviewPage() -> impl IntoView {
    view! {
        <Shell title="Overview">
            <p class="text-on-surface-variant text-sm">"Overview coming soon."</p>
        </Shell>
    }
}
