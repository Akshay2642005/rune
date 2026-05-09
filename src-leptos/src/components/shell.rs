use crate::components::hooks::use_theme_mode::use_theme_mode;
use crate::components::search_context::SearchContext;
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
        icon: "memory",
        href: "/ui/workers",
    },
    NavItem {
        label: "API Keys",
        icon: "vpn_key",
        href: "/ui/keys",
    },
];

#[component]
pub fn Shell(#[prop(into)] title: String, children: Children) -> impl IntoView {
    let auth = AuthContext::get();
    let location = use_location();
    let theme = use_theme_mode();

    let search = SearchContext::get();

    let profile_open = RwSignal::new(false);
    view! {
        <div class="bg-background text-on-background min-h-screen" style="font-size:15px">
            // ── Sidebar ───────────────────────────────────────────────────
            <aside class="fixed left-0 top-0 h-screen w-60 bg-surface-container-lowest \
                          border-r border-outline-variant flex flex-col z-50">
                <div class="px-5 py-5 border-b border-outline-variant">
                    <h3 class="text-[17px] font-bold text-on-surface leading-tight">"Rune Console"</h3>
                    <p class="text-[13px] text-secondary mt-0.5">"Infrastructure Admin"</p>
                </div>

                <nav class="flex-1 py-3">
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
                                    "flex items-center gap-3 px-5 py-2.5 text-[14px] font-semibold \
                                     text-primary border-l-[3px] border-primary bg-primary/5 \
                                     transition-all"
                                } else {
                                    "flex items-center gap-3 px-5 py-2.5 text-[14px] text-secondary \
                                     border-l-[3px] border-transparent \
                                     hover:text-on-surface hover:bg-surface-container transition-all"
                                }
                            >
                                <span class="material-symbols-outlined" style="font-size:20px">{icon}</span>
                                {label}
                            </A>
                        }
                    }).collect_view()}
                </nav>

                // ── User footer ───────────────────────────────────────────
                <div class="px-5 py-4 border-t border-outline-variant">
                    <div class="flex items-center gap-3">
                        <div class="w-9 h-9 rounded-full bg-primary-container flex items-center \
                                    justify-center text-on-primary-container text-[13px] font-bold shrink-0">
                            "RC"
                        </div>
                        <div class="flex-1 min-w-0">
                            <p class="text-[14px] font-semibold text-on-surface truncate">"Rune Console"</p>
                            <p class="text-[12px] text-secondary">"Admin"</p>
                        </div>
                        <button
                            class="p-1.5 text-secondary hover:text-error transition-colors rounded"
                            title="Sign out"
                            on:click=move |_| {
                                auth.logout();
                                let navigate = leptos_router::hooks::use_navigate();
                                navigate("/ui/login", Default::default());
                            }
                        >
                            <span class="material-symbols-outlined" style="font-size:20px">"logout"</span>
                        </button>
                    </div>
                </div>
            </aside>

            // ── Main ──────────────────────────────────────────────────────
            <main class="ml-60 min-h-screen">
                // ── Topnav ────────────────────────────────────────────────
                <header class="sticky top-0 z-40 bg-surface border-b border-outline-variant \
                               flex items-center w-full px-6 h-14 gap-4">
                    <span class="text-[18px] font-semibold text-on-surface shrink-0">{title}</span>

                    // Search bar
                    <div class="flex-1 flex justify-center">
                    <div class="w-150">
                        <div class="relative">
                            <span class="material-symbols-outlined absolute left-3 top-1/2 \
                                         -translate-y-1/2 text-secondary" style="font-size:18px">
                                "search"
                            </span>
                            <input
                                class="w-full pl-9 pr-4 py-1.5 bg-surface-container-low \
                                       border border-outline-variant rounded-lg text-[14px] \
                                       focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                                placeholder="Search resources..."
                                type="text"
                                prop:value=move || search.query.get()
                                on:input=move |e| {
                                    use leptos::ev::Event;
                                    let val = event_target_value(&e);
                                    search.query.set(val);
                                }
                            />
                        </div>
                    </div>
                    </div>

                    // Action icons
                    <div class="flex items-center gap-1 text-secondary shrink-0">
                        <button class="p-2 hover:bg-surface-container-low transition-colors rounded-lg">
                            <span class="material-symbols-outlined" style="font-size:22px">"notifications"</span>
                        </button>

                        // Profile avatar + dropdown
                        <div class="relative">
                            <button
                                class="w-8 h-8 rounded-full bg-primary-container flex items-center \
                                       justify-center text-on-primary-container text-[12px] font-bold \
                                       border border-outline-variant hover:opacity-80 transition-opacity"
                                on:click=move |_| profile_open.update(|v| *v = !*v)
                            >
                                "RC"
                            </button>

                            // Dropdown
                            {move || profile_open.get().then(|| view! {
                                <div class="absolute right-0 top-10 w-52 bg-surface-container-lowest \
                                            border border-outline-variant rounded-xl shadow-lg z-50 \
                                            overflow-hidden py-1">
                                    <div class="px-4 py-3 border-b border-outline-variant">
                                        <p class="text-[14px] font-semibold text-on-surface">"Rune Console"</p>
                                        <p class="text-[12px] text-secondary">"Admin"</p>
                                    </div>
                                    <button
                                        class="w-full flex items-center gap-3 px-4 py-2.5 text-[14px] \
                                               text-secondary hover:bg-surface-container hover:text-on-surface \
                                               transition-colors"
                                        on:click=move |_| {
                                            profile_open.set(false);
                                            theme.toggle();
                                        }
                                    >
                                        <span class="material-symbols-outlined" style="font-size:18px">
                                            {move || if theme.is_dark() { "light_mode" } else { "dark_mode" }}
                                        </span>
                                        {move || if theme.is_dark() { "Light mode" } else { "Dark mode" }}
                                    </button>
                                    <button
                                        class="w-full flex items-center gap-3 px-4 py-2.5 text-[14px] \
                                               text-error hover:bg-error/5 transition-colors"
                                        on:click=move |_| {
                                            profile_open.set(false);
                                            auth.logout();
                                            let navigate = leptos_router::hooks::use_navigate();
                                            navigate("/ui/login", Default::default());
                                        }
                                    >
                                        <span class="material-symbols-outlined" style="font-size:18px">"logout"</span>
                                        "Sign out"
                                    </button>
                                </div>
                            })}
                        </div>
                    </div>
                </header>

                <div class="p-8">
                    {children()}
                </div>
            </main>
        </div>
    }
}
