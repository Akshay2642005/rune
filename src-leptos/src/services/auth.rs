use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct AuthContext {
    pub authenticated: RwSignal<Option<bool>>, // None = pending, Some(true/false) = resolved
}
impl AuthContext {
    pub fn provide() {
        let ctx = AuthContext {
            authenticated: RwSignal::new(None),
        };
        provide_context(ctx);
    }

    pub fn provide_and_get() -> Self {
        let ctx = AuthContext {
            authenticated: RwSignal::new(None),
        };
        provide_context(ctx);
        ctx
    }

    pub fn get() -> Self {
        use_context::<AuthContext>().expect("AuthContext not provided")
    }

    pub fn set_authenticated(&self, v: bool) {
        self.authenticated.set(Some(v));
    }

    pub fn logout(&self) {
        self.authenticated.set(Some(false));
        leptos::task::spawn_local(async {
            let _ = crate::services::api::logout().await;
        });
    }
}
