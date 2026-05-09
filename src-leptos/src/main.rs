#![allow(non_snake_case)]
mod app;
mod components;
mod pages;
mod services;
mod utils;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(crate::app::App);
}
