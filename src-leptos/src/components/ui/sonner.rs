use leptos::prelude::*;

pub use super::toast_context::{Toast, ToastContext, ToastVariant};

#[component]
pub fn Toaster() -> impl IntoView {
    let ctx = ToastContext::get();

    view! {
        <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 items-end pointer-events-none">
            <For
                each=move || ctx.toasts().get()
                key=|t| t.id
                children=move |toast| {
                    let id = toast.id;
                    let ctx2 = ctx;
                    let (bg, icon) = match toast.variant {
                        ToastVariant::Success => (
                            "bg-surface-container-lowest border border-outline-variant",
                            view! { <span class="material-symbols-outlined text-[18px] text-success">"check_circle"</span> },
                        ),
                        ToastVariant::Error => (
                            "bg-surface-container-lowest border border-error/30",
                            view! { <span class="material-symbols-outlined text-[18px] text-error">"error"</span> },
                        ),
                        ToastVariant::Warning => (
                            "bg-surface-container-lowest border border-warning/30",
                            view! { <span class="material-symbols-outlined text-[18px] text-warning">"warning"</span> },
                        ),
                        ToastVariant::Default => (
                            "bg-surface-container-lowest border border-outline-variant",
                            view! { <span class="material-symbols-outlined text-[18px] text-on-surface-variant">"info"</span> },
                        ),
                    };

                    view! {
                        <div class=format!(
                            "pointer-events-auto flex items-start gap-3 w-80 rounded-xl shadow-card px-4 py-3 {}",
                            bg
                        )>
                            {icon}
                            <div class="flex-1 min-w-0">
                                <p class="text-sm font-medium text-on-surface">{toast.title}</p>
                                {toast.description.map(|d| view! {
                                    <p class="text-xs text-on-surface-variant mt-0.5">{d}</p>
                                })}
                            </div>
                            <button
                                class="shrink-0 text-on-surface-variant/60 hover:text-on-surface transition-colors"
                                on:click=move |_| ctx2.dismiss(id)
                            >
                                <span class="material-symbols-outlined text-[16px]">"close"</span>
                            </button>
                        </div>
                    }
                }
            />
        </div>
    }
}
