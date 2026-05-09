use crate::components::search_context::SearchContext;
use crate::components::shell::Shell;
use crate::services::api::{list_functions, list_keys};
use crate::services::types::FunctionRecord;
use leptos::prelude::*;

#[component]
pub fn OverviewPage() -> impl IntoView {
    let functions = LocalResource::new(|| async { list_functions().await });
    let keys      = LocalResource::new(|| async { list_keys().await });
    let search    = SearchContext::get();

    view! {
        <Shell title="Edge Compute">
            // ── Breadcrumb ────────────────────────────────────────────────
            <div class="flex items-center gap-sm mb-lg text-secondary font-label-caps uppercase">
                <span>"Infrastructure"</span>
                <span class="material-symbols-outlined text-[12px]">"chevron_right"</span>
                <span class="text-on-surface">"Global Dashboard"</span>
            </div>

            // ── Bento grid ────────────────────────────────────────────────
            <div class="grid grid-cols-12 gap-gutter">

                // ── Left: metrics + table ─────────────────────────────────
                <div class="col-span-12 lg:col-span-8 flex flex-col gap-gutter">

                    // Stat cards row
                    <div class="grid grid-cols-3 gap-gutter">
                        <StatCard
                            label="Total Workers"
                            suffix=""
                            value=Signal::derive(move || {
                                functions.get().and_then(|r| r.ok())
                                    .map(|v| v.len().to_string())
                                    .unwrap_or_else(|| "—".into())
                            })
                        />
                        <StatCard
                            label="API Keys"
                            suffix=""
                            value=Signal::derive(move || {
                                keys.get().and_then(|r| r.ok())
                                    .map(|v| v.len().to_string())
                                    .unwrap_or_else(|| "—".into())
                            })
                        />
                        <StatCard
                            label="Avg CPU Time"
                            suffix="ms"
                            value=Signal::derive(|| "< 1".into())
                        />
                    </div>

                    // Recent Deployments table
                    <div class="bg-surface-container-lowest border border-outline-variant \
                                rounded shadow-card overflow-hidden">
                        <div class="px-5 py-3 border-b border-outline-variant \
                                    flex justify-between items-center">
                            <h3 class="text-[16px] font-semibold text-on-surface">"Recent Deployments"</h3>
                            <a href="/ui/workers"
                               class="text-primary font-bold text-[13px] flex items-center gap-xs">
                                "View History"
                                <span class="material-symbols-outlined" style="font-size:15px">"open_in_new"</span>
                            </a>
                        </div>
                        <div class="overflow-x-auto">
                            <table class="w-full text-left">
                                <thead>
                                    <tr class="bg-surface-container-low/50">
                                        <th class="px-5 py-2 text-[12px] font-bold tracking-wide text-secondary \
                                                   border-b border-outline-variant uppercase">"Deployment ID"</th>
                                        <th class="px-5 py-2 text-[12px] font-bold tracking-wide text-secondary \
                                                   border-b border-outline-variant uppercase">"Status"</th>
                                        <th class="px-5 py-2 text-[12px] font-bold tracking-wide text-secondary \
                                                   border-b border-outline-variant uppercase">"Route"</th>
                                        <th class="px-5 py-2 text-[12px] font-bold tracking-wide text-secondary \
                                                   border-b border-outline-variant uppercase">"Subdomain"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <Suspense fallback=|| view! {
                                        <tr><td colspan="4" class="px-5 py-4 text-center text-[14px] text-secondary">"Loading…"</td></tr>
                                    }>
                                        {move || functions.get().map(|result| match result {
                                            Err(e) => view! {
                                                <tr><td colspan="4" class="px-5 py-4 text-center text-[14px] text-error">{e}</td></tr>
                                            }.into_any(),
                                            Ok(list) if list.is_empty() => view! {
                                                <tr><td colspan="4" class="px-5 py-4 text-center text-[14px] text-secondary">"No functions deployed yet."</td></tr>
                                            }.into_any(),
                                            Ok(list) => view! {
                                                <For
                                                    each=move || {
                                                        let q = search.query.get().to_lowercase();
                                                        list.clone().into_iter().filter(move |f| {
                                                            q.is_empty()
                                                                || f.id.to_lowercase().contains(&q)
                                                                || f.route.to_lowercase().contains(&q)
                                                                || f.subdomain.as_deref().unwrap_or("").to_lowercase().contains(&q)
                                                        }).collect::<Vec<_>>()
                                                    }
                                                    key=|f| f.id.clone()
                                                    children=|f: FunctionRecord| view! {
                                                        <tr class="border-t border-outline-variant \
                                                                   hover:bg-surface-container-low/50 \
                                                                   transition-colors">
                                                            <td class="px-5 py-2 text-[14px] font-mono text-primary">{f.id}</td>
                                                            <td class="px-5 py-2">
                                                                <span class="inline-flex items-center gap-1 px-2 py-0.5 \
                                                                             bg-emerald-100 text-emerald-800 rounded-full \
                                                                             text-[12px] font-bold">
                                                                    <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 inline-block"></span>
                                                                    "ACTIVE"
                                                                </span>
                                                            </td>
                                                            <td class="px-5 py-2 text-[14px] text-secondary">{f.route}</td>
                                                            <td class="px-5 py-2 text-[14px] text-secondary">
                                                                {f.subdomain.unwrap_or_else(|| "—".into())}
                                                            </td>
                                                        </tr>
                                                    }
                                                />
                                            }.into_any(),
                                        })}
                                    </Suspense>
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>

                // ── Right: Quick Start + Usage ────────────────────────────
                <div class="col-span-12 lg:col-span-4 flex flex-col gap-gutter">
                    // Quick Start dark card
                    <div class="bg-[#111827] rounded-xl p-md text-white shadow-lg overflow-hidden relative">
                        <div class="absolute top-0 right-0 w-32 h-32 bg-primary/20 blur-3xl \
                                    rounded-full translate-x-1/2 -translate-y-1/2"></div>
                        <h3 class="font-h3 text-h3 mb-sm flex items-center gap-xs">
                            <span class="material-symbols-outlined text-primary-fixed-dim \
                                         [font-variation-settings:'FILL'_1]">"bolt"</span>
                            "Quick Start"
                        </h3>
                        <p class="text-body-sm text-slate-400 mb-md">
                            "Deploy your first edge worker in seconds using the CLI or SDK."
                        </p>
                        <div class="space-y-xs">
                            <div class="bg-slate-900/50 rounded p-sm border border-slate-800">
                                <p class="font-label-caps text-slate-500 text-[10px] mb-xs">"TERMINAL"</p>
                                <code class="font-mono text-emerald-400 text-[12px] block">
                                    "$ rune deploy ./worker.wasm"
                                </code>
                            </div>
                            <div class="bg-slate-900/50 rounded p-sm border border-slate-800">
                                <p class="font-label-caps text-slate-500 text-[10px] mb-xs">"WORKER.RS"</p>
                                <code class="font-mono text-[12px] block text-blue-300">
                                    <span class="text-purple-400">"pub fn"</span>
                                    " handle(req: Request) -> Response {"
                                    <br/>
                                    "  Response::ok("
                                    <span class="text-emerald-400">"\"Hello Rune!\""</span>
                                    ")"
                                    <br/>
                                    "}"
                                </code>
                            </div>
                        </div>
                        <a href="https://github.com/Akshay2642005/rune"
                           target="_blank"
                           class="block w-full mt-md bg-primary-container text-white py-sm \
                                  rounded text-center font-bold text-body-sm \
                                  hover:opacity-90 transition-opacity">
                            "View Documentation"
                        </a>
                    </div>

                    // Usage quota card
                    <div class="bg-surface-container-lowest border border-outline-variant \
                                rounded p-md shadow-card">
                        <h3 class="font-h3 text-h3 text-on-surface mb-md">"Usage Quota"</h3>
                        <div class="space-y-md">
                            <div>
                                <div class="flex justify-between text-body-sm mb-xs">
                                    <span class="text-secondary">"Workers"</span>
                                    <span class="font-bold text-on-surface">
                                        {move || {
                                            let n = functions.get().and_then(|r| r.ok())
                                                .map(|v| v.len()).unwrap_or(0);
                                            format!("{} / 100", n)
                                        }}
                                    </span>
                                </div>
                                <div class="h-1.5 w-full bg-surface-container-low rounded-full overflow-hidden">
                                    <div class="bg-primary h-full" style="width: 10%"></div>
                                </div>
                            </div>
                            <div>
                                <div class="flex justify-between text-body-sm mb-xs">
                                    <span class="text-secondary">"API Keys"</span>
                                    <span class="font-bold text-on-surface">
                                        {move || {
                                            let n = keys.get().and_then(|r| r.ok())
                                                .map(|v| v.len()).unwrap_or(0);
                                            format!("{} / 10", n)
                                        }}
                                    </span>
                                </div>
                                <div class="h-1.5 w-full bg-surface-container-low rounded-full overflow-hidden">
                                    <div class="bg-emerald-500 h-full" style="width: 20%"></div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            // ── FAB ───────────────────────────────────────────────────────
            <div class="fixed bottom-lg right-lg z-50">
                <a href="/ui/workers"
                   class="bg-primary shadow-xl text-white h-14 px-lg rounded-full \
                          flex items-center gap-sm font-bold \
                          hover:scale-105 active:scale-95 transition-all">
                    <span class="material-symbols-outlined">"add"</span>
                    "Deploy New Worker"
                </a>
            </div>
        </Shell>
    }
}

#[component]
fn StatCard(label: &'static str, suffix: &'static str, value: Signal<String>) -> impl IntoView {
    view! {
        <div class="col-span-1 bg-surface-container-lowest border border-outline-variant \
                    rounded p-md flex flex-col gap-sm shadow-card">
            <span class="font-label-caps text-secondary uppercase text-[11px]">{label}</span>
            <div class="flex items-baseline gap-xs">
                <span class="font-h1 text-h1 text-on-surface">{value}</span>
                <span class="text-secondary font-body-sm">{suffix}</span>
            </div>
        </div>
    }
}
