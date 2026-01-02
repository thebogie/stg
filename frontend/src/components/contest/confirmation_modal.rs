use yew::prelude::*;
use shared::dto::contest::ContestDto;
use crate::components::contest::confirmation::ContestConfirmation;

#[derive(Properties, Clone, PartialEq)]
pub struct ContestConfirmationModalProps {
    pub contest: Option<ContestDto>,
    pub is_open: bool,
    pub on_confirm: Callback<()>,
    pub on_cancel: Callback<()>,
    pub on_edit: Callback<()>,
}

#[function_component(ContestConfirmationModal)]
pub fn contest_confirmation_modal(props: &ContestConfirmationModalProps) -> Html {
    let props = props.clone();

    let on_confirm = {
        let on_confirm = props.on_confirm.clone();
        Callback::from(move |_: ()| {
            gloo::console::log!("ContestConfirmationModal: Confirm callback triggered");
            gloo::console::log!("ContestConfirmationModal: Emitting on_confirm callback");
            on_confirm.emit(());
            gloo::console::log!("ContestConfirmationModal: on_confirm callback emitted");
        })
    };

    let on_cancel = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_| {
            on_cancel.emit(());
        })
    };

    // Create separate callbacks for the ContestConfirmation component
    let confirmation_on_cancel = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_: ()| {
            on_cancel.emit(());
        })
    };

    let confirmation_on_edit = {
        let on_edit = props.on_edit.clone();
        Callback::from(move |_: ()| {
            on_edit.emit(());
        })
    };

    let on_overlay_click = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_| {
            on_cancel.emit(());
        })
    };

    if !props.is_open || props.contest.is_none() {
        return html! {};
    }

    let contest = props.contest.unwrap();

    html! {
        <div class="fixed inset-0 bg-black bg-opacity-50 backdrop-blur-sm flex items-center justify-center z-50" onclick={on_overlay_click}>
            <div class="bg-white rounded-xl shadow-xl w-full max-w-4xl mx-4 transform transition-all max-h-[90vh] overflow-y-auto" onclick={|e: MouseEvent| e.stop_propagation()}>
                <div class="flex justify-between items-center p-6 border-b border-gray-200">
                    <h2 class="text-2xl font-semibold text-gray-800">{"Confirm Contest Details"}</h2>
                    <button 
                        class="text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-full p-2 transition-colors duration-200" 
                        onclick={on_cancel.clone()}
                    >
                        {"×"}
                    </button>
                </div>
                <div class="p-6">
                    <ContestConfirmation 
                        contest={contest}
                        on_confirm={on_confirm}
                        on_cancel={confirmation_on_cancel}
                        on_edit={confirmation_on_edit}
                    />
                </div>
            </div>
        </div>
    }
}
