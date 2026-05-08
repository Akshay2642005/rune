use crate::components::hooks::use_theme_mode::use_theme_mode;
use crate::services::auth::AuthContext;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

#[derive(Clone)]
struct NavItem {
    label: &'static str,
    icon: &'static str,
    href: &'static str,
}

const NAV: &[NavItem] = &[
    NavItem {
        label: "Overview",
        icon: "dashboard",
        href: "/ui/overview",
    },
    NavItem {
        label: "Workers",
        icon: "bolt",
        href: "/ui/workers",
    },
    NavItem {
        label: "API Keys",
        icon: "key",
        href: "/ui/keys",
    },
];

#[component]
pub fn Shell(#[prop(into)] title: String, children: Children) -> impl IntoView {
    let auth = AuthContext::get();
    let location = use_location();
    let theme = use_theme_mode();

    view! {
        <div class="flex h-screen bg-background text-on-surface overflow-hidden">
            <aside class="w-60 shrink-0 flex flex-col bg-surface-container-low \
                          border-r border-outline-variant">
                <div class="flex items-center gap-2 px-4 h-14 border-b border-outline-variant">
                    <span class="material-symbols-outlined text-primary \
                                 [font-variation-settings:'FILL'_1]">"memory"</span>
                    <span class="font-semibold text-sm">"Rune Console"</span>
                </div>

                <nav class="flex-1 px-2 py-3 flex flex-col gap-0.5">
                    {NAV.iter().map(|item| {
                        let href = item.href;
                        let icon = item.icon;
                        let label = item.label;
                        let active = {
                            let href_s = href.to_string();
                            move || location.pathname.get().starts_with(&href_s)
                        };
                        view! {
                            <A
                                href=href
                                attr:class=move || if active() {
                                    "flex items-center gap-3 px-3 py-2 rounded-lg text-sm \
                                     bg-primary/10 text-primary font-medium relative \
                                     before:absolute before:left-0 before:top-2 before:bottom-2 \
                                     before:w-[3px] before:bg-primary before:rounded-full"
                                } else {
                                    "flex items-center gap-3 px-3 py-2 rounded-lg text-sm \
                                     text-on-surface-variant hover:bg-surface-container \
                                     hover:text-on-surface transition-colors"
                                }
                            >
                                <span class="material-symbols-outlined text-[20px]">{icon}</span>
                                {label}
                            </A>
                        }
                    }).collect_view()}
                </nav>

                <div class="px-2 py-3 border-t border-outline-variant">
                    <button
                        class="flex items-center gap-3 px-3 py-2 w-full rounded-lg text-sm \
                               text-on-surface-variant hover:bg-surface-container \
                               hover:text-error transition-colors"
                        on:click=move |_| {
                            auth.logout();
                            let navigate = leptos_router::hooks::use_navigate();
                            navigate("/ui/login", Default::default());
                        }
                    >
                        <span class="material-symbols-outlined text-[20px]">"logout"</span>
                        "Sign out"
                    </button>
                </div>
            </aside>

            <div class="flex-1 flex flex-col min-w-0">
                <header class="h-14 shrink-0 flex items-center justify-between px-6 \
                               border-b border-outline-variant bg-surface">
                    <h1 class="font-semibold text-base">{title}</h1>
                    <div class="flex items-center gap-2">
                        <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg \
                                    bg-surface-container border border-outline-variant \
                                    text-sm text-on-surface-variant w-48">
                            <span class="material-symbols-outlined text-[16px]">"search"</span>
                            <span>"Search…"</span>
                        </div>
                        <button class="p-2 rounded-lg hover:bg-surface-container transition-colors">
                            <span class="material-symbols-outlined text-[20px] \
                                         text-on-surface-variant">"notifications"</span>
                        </button>
                        <button
                            class="p-2 rounded-lg hover:bg-surface-container transition-colors"
                            on:click=move |_| theme.toggle()
                            title=move || if theme.is_dark() { "Switch to light mode" } else { "Switch to dark mode" }
                        >
                            <span class="material-symbols-outlined text-[20px] text-on-surface-variant">
                                {move || if theme.is_dark() { "light_mode" } else { "dark_mode" }}
                            </span>
                        </button>
                        <button class="p-2 rounded-lg hover:bg-surface-container transition-colors">
                            <span class="material-symbols-outlined text-[20px] \
                                         text-on-surface-variant">"settings"</span>
                        </button>
                    </div>
                </header>

                <main class="flex-1 overflow-auto p-6">

                    {children()}
                </main>
            </div>
        </div>
    }
}
