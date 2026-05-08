#![allow(non_snake_case)]
mod components;
mod utils;

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center h-screen bg-gray-50">
            <h1 class="text-2xl font-semibold text-gray-800">"Hello Rune"</h1>
        </div>
    }
}
