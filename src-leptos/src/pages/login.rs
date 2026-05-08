use crate::components::ui::toast_context::{ToastContext, ToastVariant};
use crate::services::api::login as api_login;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn LoginPage() -> impl IntoView {
    let toast = ToastContext::get();
    let token = RwSignal::new(String::new());
    let loading = RwSignal::new(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let key = token.get();
        if key.is_empty() {
            toast.push("Invalid Token", Some("Token must not be empty".into()), ToastVariant::Error);
            return;
        }
        loading.set(true);
        spawn_local(async move {
            match api_login(&key).await {
                Ok(()) => {
                    let _ = web_sys::window().unwrap().location().set_href("/ui/overview");
                }
                Err(e) => {
                    loading.set(false);
                    toast.push("Authentication Failed", Some(e), ToastVariant::Error);
                }
            }
        });
    };

    view! {
        // Ambient blobs
        <div aria-hidden="true"
             class="fixed inset-0 z-0 overflow-hidden opacity-10 pointer-events-none">
            <div class="absolute -top-1/2 -left-1/4 w-full h-full \
                        bg-primary-container rounded-full blur-[120px]"/>
            <div class="absolute -bottom-1/2 -right-1/4 w-full h-full \
                        bg-secondary-container rounded-full blur-[120px]"/>
        </div>

        <main class="relative z-10 flex flex-col items-center justify-center min-h-screen px-4">
            // Branding
            <div class="mb-8 text-center">
                <div class="flex items-center justify-center gap-2 mb-2">
                    <span class="material-symbols-outlined text-primary text-[40px] \
                                 [font-variation-settings:'FILL'_1]">"memory"</span>
                    <h1 class="text-2xl font-semibold text-on-surface">"Rune Console"</h1>
                </div>
                <p class="text-sm text-on-surface-variant">"control plane for rune"</p>
            </div>

            // Card
            <div class="w-full max-w-[440px] bg-surface-container-lowest border \
                        border-outline-variant rounded-xl shadow-card p-8 flex flex-col gap-6">
                <div>
                    <h2 class="font-semibold text-on-surface">"Administrative Login"</h2>
                    <p class="text-sm text-on-surface-variant mt-1">
                        "Access the rune control panel."
                    </p>
                </div>

                <form class="flex flex-col gap-4" on:submit=on_submit>
                    <div class="flex flex-col gap-1.5">
                        <label class="text-sm font-medium text-on-surface" for="token">
                            "Admin Token or System Key"
                        </label>
                        <input
                            id="token"
                            type="password"
                            placeholder="rune_sk_…"
                            class="w-full px-3 py-2 rounded-lg border border-outline-variant \
                                   bg-surface text-on-surface text-sm \
                                   focus:outline-none focus:ring-2 focus:ring-primary/40 \
                                   focus:border-primary transition-colors"
                            prop:value=token
                            on:input=move |ev| token.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="flex items-center justify-between">
                        <label class="flex items-center gap-2 cursor-pointer text-sm \
                                      text-on-surface-variant">
                            <input type="checkbox" class="rounded border-outline-variant \
                                                          text-primary focus:ring-primary"/>
                            "Keep session active"
                        </label>
                        <a href="#" class="text-sm text-primary hover:underline">"Forgot key?"</a>
                    </div>

                    <button
                        type="submit"
                        disabled=move || loading.get()
                        class="flex items-center justify-center gap-2 w-full py-2 px-4 \
                               bg-primary text-on-primary rounded-lg text-sm font-medium \
                               hover:bg-primary/90 disabled:opacity-60 disabled:cursor-not-allowed \
                               transition-colors"
                    >
                        <Show
                            when=move || loading.get()
                            fallback=|| view! {
                                <span class="material-symbols-outlined text-[18px]">"login"</span>
                                "Authenticate"
                            }
                        >
                            <span class="material-symbols-outlined text-[18px] animate-spin">
                                "autorenew"
                            </span>
                            "Authenticating…"
                        </Show>
                    </button>
                </form>

                // Security notice
                <div class="flex gap-3 p-3 bg-surface-container border border-outline-variant \
                            rounded-lg">
                    <span class="material-symbols-outlined text-[20px] text-on-surface-variant shrink-0">
                        "verified_user"
                    </span>
                    <p class="text-xs text-on-surface-variant">
                        "This session is protected by end-to-end hardware encryption. \
                         Unauthorized access attempts are logged."
                    </p>
                </div>
            </div>

            // Footer
            <footer class="mt-8 flex gap-6">
                {["Documentation", "Status", "Security Policy"].iter().map(|l| view! {
                    <a href="#"
                       class="text-xs font-bold uppercase tracking-wider text-on-surface-variant \
                              hover:text-primary transition-colors">
                        {*l}
                    </a>
                }).collect_view()}
            </footer>
        </main>
    }
}
