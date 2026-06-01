use crate::components::contest::confirmation::ContestConfirmation;
use shared::dto::contest::ContestDto;
use yew::prelude::*;

#[derive(Properties, Clone, PartialEq)]
pub struct ContestConfirmationModalProps {
    pub contest: Option<ContestDto>,
    /// Logged-in player handle (or empty) shown as contest creator before submit.
    pub creator_display: String,
    #[prop_or_default]
    pub image_preview_url: Option<String>,
    pub is_open: bool,
    pub on_confirm: Callback<()>,
    pub on_cancel: Callback<()>,
    pub on_edit: Callback<()>,
}

#[function_component(ContestConfirmationModal)]
pub fn contest_confirmation_modal(props: &ContestConfirmationModalProps) -> Html {
    let props = props.clone();

    let on_confirm_click = {
        let on_confirm = props.on_confirm.clone();
        Callback::from(move |_: MouseEvent| {
            on_confirm.emit(());
        })
    };

    let on_cancel = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_| {
            on_cancel.emit(());
        })
    };

    let on_edit = {
        let on_edit = props.on_edit.clone();
        Callback::from(move |_: MouseEvent| {
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
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm px-3 py-4 sm:px-4" onclick={on_overlay_click}>
            <div
                class="flex max-h-[min(92vh,48rem)] w-full max-w-2xl flex-col overflow-hidden rounded-xl bg-white shadow-xl"
                onclick={|e: MouseEvent| e.stop_propagation()}
            >
                <div class="flex shrink-0 items-center justify-between border-b border-gray-200 px-4 py-3 sm:px-5">
                    <h2 class="text-lg font-semibold text-gray-800 sm:text-xl">{"Review contest"}</h2>
                    <button
                        type="button"
                        class="rounded-full p-1.5 text-gray-500 transition-colors hover:bg-gray-100 hover:text-gray-700"
                        onclick={on_cancel.clone()}
                    >
                        {"×"}
                    </button>
                </div>

                <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 py-3 sm:px-5">
                    <p class="mb-3 text-xs text-gray-500 sm:text-sm">{"Check the details below, then confirm to submit."}</p>
                    <ContestConfirmation
                        contest={contest}
                        creator_display={props.creator_display.clone()}
                        image_preview_url={props.image_preview_url.clone()}
                    />
                </div>

                <div class="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-gray-200 bg-gray-50 px-4 py-3 sm:px-5">
                    <button
                        type="button"
                        onclick={on_edit}
                        class="btn-material-secondary inline-flex items-center px-3 py-2 text-sm"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="mr-1 h-4 w-4 shrink-0" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
                            <path fill-rule="evenodd" d="M9.707 14.707a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414l4-4a1 1 0 011.414 1.414L7.414 9H15a1 1 0 110 2H7.414l2.293 2.293a1 1 0 010 1.414z" clip-rule="evenodd" />
                        </svg>
                        {"Back to edit"}
                    </button>
                    <button
                        type="button"
                        onclick={on_confirm_click}
                        class="btn-material-primary px-4 py-2 text-sm font-medium"
                    >
                        {"Confirm & record contest"}
                    </button>
                </div>
            </div>
        </div>
    }
}
