use leptos::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ToastVariant {
    Default,
    Success,
    Error,
    Warning,
}

#[derive(Clone)]
pub struct Toast {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub variant: ToastVariant,
}

#[derive(Clone, Copy)]
pub struct ToastContext {
    toasts: RwSignal<Vec<Toast>>,
    next_id: RwSignal<u64>,
}

impl ToastContext {
    pub fn provide() {
        let ctx = Self {
            toasts: RwSignal::new(vec![]),
            next_id: RwSignal::new(0),
        };
        provide_context(ctx);
    }

    pub fn get() -> Self {
        use_context::<Self>().expect("ToastContext not provided")
    }

    pub fn push(&self, title: impl Into<String>, description: Option<String>, variant: ToastVariant) {
        let id = self.next_id.get_untracked();
        self.next_id.set(id + 1);
        let toast = Toast { id, title: title.into(), description, variant };
        self.toasts.update(|v| v.push(toast));

        // Auto-dismiss after 4 s
        let toasts = self.toasts;
        set_timeout(
            move || toasts.update(|v| v.retain(|t| t.id != id)),
            std::time::Duration::from_millis(4000),
        );
    }

    pub fn dismiss(&self, id: u64) {
        self.toasts.update(|v| v.retain(|t| t.id != id));
    }

    pub fn toasts(&self) -> RwSignal<Vec<Toast>> {
        self.toasts
    }
}
