use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct ModalProps {
    pub is_open: bool,
    pub title: String,
    pub message: String,
    pub on_close: Callback<()>,
    pub button_class: String,
    pub button_text: String,
}

#[function_component(Modal)]
pub fn modal(props: &ModalProps) -> Html {
    if !props.is_open {
        return html! {};
    }

    let on_overlay_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_close.emit(());
        })
    };

    let on_modal_click = {
        Callback::from(|e: MouseEvent| {
            e.stop_propagation();
        })
    };

    let on_button_click = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    html! {
        <div class="fixed inset-0 z-50 flex items-center justify-center">
            <div 
                class="absolute inset-0 bg-black bg-opacity-50"
                onclick={on_overlay_click}
            ></div>
            <div 
                class="relative bg-white rounded-lg shadow-xl p-6 max-w-md w-full mx-4"
                onclick={on_modal_click}
            >
                <div class="mb-4">
                    <h3 class="text-lg font-medium text-gray-900">{&props.title}</h3>
                </div>
                <div class="mb-6">
                    <p class="text-sm text-gray-600">{&props.message}</p>
                </div>
                <div class="flex justify-end">
                    <button
                        onclick={on_button_click}
                        class={classes!(
                            "px-4", "py-2", "text-sm", "font-medium", "text-white", "rounded-md", "focus:outline-none", "focus:ring-2", "focus:ring-offset-2",
                            &props.button_class
                        )}
                    >
                        {&props.button_text}
                    </button>
                </div>
            </div>
        </div>
    }
} 