use crate::components::ui::sonner::Toaster;
use crate::components::ui::toast_context::ToastContext;
use crate::components::search_context::SearchContext;
use crate::pages::{
    dashboard::{KeysPage, WorkersPage},
    login::LoginPage,
    overview::OverviewPage,
};
use crate::services::{api::probe_auth, auth::AuthContext};
use crate::components::hooks::use_theme_mode::ThemeMode;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

#[component]
pub fn App() -> impl IntoView {
    let auth = AuthContext::provide_and_get();
    ToastContext::provide();
    SearchContext::provide();
    let theme = ThemeMode::init();

    // Apply/remove "dark" class on <html> whenever theme changes
    Effect::new(move |_| {
        let cl = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .document_element()
            .unwrap()
            .class_list();
        if theme.is_dark() { let _ = cl.add_1("dark"); } else { let _ = cl.remove_1("dark"); }
    });

    spawn_local(async move {
        let ok = probe_auth().await.is_ok();
        auth.set_authenticated(ok);
    });

    view! {
        <Router>
            <Routes fallback=|| view! { <RedirectToOverview/> }>
                <Route path=path!("/ui/login")    view=LoginPage/>
                <Route path=path!("/ui/overview") view=move || view! { <AuthGuard><OverviewPage/></AuthGuard> }/>
                <Route path=path!("/ui/workers")  view=move || view! { <AuthGuard><WorkersPage/></AuthGuard> }/>
                <Route path=path!("/ui/keys")     view=move || view! { <AuthGuard><KeysPage/></AuthGuard> }/>
                <Route path=path!("/ui")          view=move || view! { <RedirectToOverview/> }/>
                <Route path=path!("/")            view=move || view! { <RedirectToOverview/> }/>
            </Routes>
        </Router>
        <Toaster/>
    }
}

#[component]
fn RedirectToOverview() -> impl IntoView {
    let auth = AuthContext::get();
    view! {
        {move || match auth.authenticated.get() {
            None => view! { <></> }.into_any(),  // still probing
            Some(true) => {
                let _ = leptos_router::hooks::use_navigate()("/ui/overview", Default::default());
                view! { <></> }.into_any()
            }
            Some(false) => {
                let _ = leptos_router::hooks::use_navigate()("/ui/login", Default::default());
                view! { <></> }.into_any()
            }
        }}
    }
}

/// Reads the already-resolved auth signal — no extra network call.
#[component]
fn AuthGuard(children: ChildrenFn) -> impl IntoView {
    let auth = AuthContext::get();

    view! {
        {move || match auth.authenticated.get() {
            None => view! { <></> }.into_any(),  // show nothing while pending
            Some(false) => {
                let _ = leptos_router::hooks::use_navigate()("/ui/login", Default::default());
                view! { <></> }.into_any()
            }
            Some(true) => children().into_any(),
        }}
    }
}
