use gloo_timers::callback::Timeout;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastType {
    fn classes(&self) -> &'static str {
        match self {
            ToastType::Success => "bg-green-500 border-green-600",
            ToastType::Error => "bg-red-500 border-red-600",
            ToastType::Warning => "bg-yellow-500 border-yellow-600",
            ToastType::Info => "bg-blue-500 border-blue-600",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ToastType::Success => "✓",
            ToastType::Error => "✕",
            ToastType::Warning => "⚠",
            ToastType::Info => "ℹ",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: Uuid,
    pub message: String,
    pub toast_type: ToastType,
    pub duration: Option<u32>, // milliseconds, None for manual dismiss
}

impl Toast {
    pub fn new(message: String, toast_type: ToastType) -> Self {
        Self {
            id: Uuid::new_v4(),
            message,
            toast_type,
            duration: Some(5000), // 5 seconds default
        }
    }

    pub fn with_duration(mut self, duration: u32) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn persistent(mut self) -> Self {
        self.duration = None;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToastContext {
    pub toasts: Vec<Toast>,
    pub add_toast: Callback<Toast>,
    pub remove_toast: Callback<Uuid>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct ToastProviderProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(ToastProvider)]
pub fn toast_provider(props: &ToastProviderProps) -> Html {
    let toasts = use_state(Vec::new);

    let add_toast = {
        let toasts = toasts.clone();
        Callback::from(move |toast: Toast| {
            let toasts = toasts.clone();
            let toast_id = toast.id;
            let duration = toast.duration;

            // Add toast to list
            toasts.set({
                let mut current = (*toasts).clone();
                current.push(toast);
                current
            });

            // Auto-remove after duration if specified
            if let Some(duration_ms) = duration {
                let toasts = toasts.clone();
                let timeout = Timeout::new(duration_ms, move || {
                    toasts.set({
                        let mut current = (*toasts).clone();
                        current.retain(|t| t.id != toast_id);
                        current
                    });
                });
                timeout.forget(); // Let it run in background
            }
        })
    };

    let remove_toast = {
        let toasts = toasts.clone();
        Callback::from(move |id: Uuid| {
            toasts.set({
                let mut current = (*toasts).clone();
                current.retain(|t| t.id != id);
                current
            });
        })
    };

    let context = ToastContext {
        toasts: (*toasts).clone(),
        add_toast,
        remove_toast,
    };

    html! {
        <ContextProvider<ToastContext> context={context}>
            <div class="toast-container">
                {props.children.clone()}
                <ToastList />
            </div>
        </ContextProvider<ToastContext>>
    }
}

#[function_component(ToastList)]
fn toast_list() -> Html {
    let toast_context = use_context::<ToastContext>().expect("Toast context not found");

    html! {
        <div class="fixed top-4 right-4 z-50 space-y-2">
            {toast_context.toasts.iter().map(|toast| {
                html! {
                    <ToastItem key={toast.id.to_string()} toast={toast.clone()} />
                }
            }).collect::<Html>()}
        </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
struct ToastItemProps {
    toast: Toast,
}

#[function_component(ToastItem)]
fn toast_item(props: &ToastItemProps) -> Html {
    let toast_context = use_context::<ToastContext>().expect("Toast context not found");
    let visible = use_state(|| false);

    // Animate in
    {
        let visible = visible.clone();
        use_effect_with((), move |_| {
            let visible = visible.clone();
            let timeout = Timeout::new(10, move || {
                visible.set(true);
            });
            timeout.forget();
            || {}
        });
    }

    let on_close = {
        let toast_context = toast_context.clone();
        let toast_id = props.toast.id;
        Callback::from(move |_: MouseEvent| {
            toast_context.remove_toast.emit(toast_id);
        })
    };

    let toast_type_classes = props.toast.toast_type.classes();
    let icon = props.toast.toast_type.icon();

    html! {
        <div class={classes!(
            "transform", "transition-all", "duration-300", "ease-in-out",
            if *visible { "translate-x-0 opacity-100" } else { "translate-x-full opacity-0" }
        )}>
            <div class={classes!(
                "flex", "items-center", "p-4", "rounded-lg", "shadow-lg", "border-l-4", "text-white", "min-w-80", "max-w-md",
                toast_type_classes
            )}>
                <div class="flex-shrink-0 mr-3">
                    <span class="text-lg font-bold">{icon}</span>
                </div>
                <div class="flex-1">
                    <p class="text-sm font-medium">{&props.toast.message}</p>
                </div>
                <div class="flex-shrink-0 ml-3">
                    <button
                        onclick={on_close}
                        class="text-white hover:text-gray-200 focus:outline-none focus:text-gray-200 transition-colors duration-200"
                    >
                        <span class="text-lg">{"×"}</span>
                    </button>
                </div>
            </div>
        </div>
    }
}

// Helper function to show a toast
pub fn show_toast(message: &str, _toast_type: ToastType) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Ok(Some(_toast_container)) = document.query_selector(".toast-container") {
                // This is a fallback for when context is not available
                // In practice, you'd use the context
                gloo::console::log!("Toast:", message);
            }
        }
    }
}
