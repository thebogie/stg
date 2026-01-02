use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct SmartClickableProps {
    pub text: String,
    pub click_type: ClickType,
    pub on_click: Callback<()>,
    pub class: Option<String>,
}

#[derive(PartialEq, Clone)]
pub enum ClickType {
    ContestsModal,           // Opens contests modal
    BggLink,                // Links to BoardGameGeek (game_id passed separately)
    ExternalLink,           // Links to external URL (url passed separately)
    InternalLink,           // Internal navigation (route passed separately)
    CustomAction,           // Custom action (like opening a different modal)
}

#[function_component(SmartClickable)]
pub fn smart_clickable(props: &SmartClickableProps) -> Html {
    let base_classes = "font-medium text-blue-600 hover:text-blue-800 hover:underline cursor-pointer";
    let final_classes = if let Some(ref custom_class) = props.class {
        format!("{} {}", base_classes, custom_class)
    } else {
        base_classes.to_string()
    };

    match &props.click_type {
        ClickType::ContestsModal => {
            html! {
                <button
                    class={final_classes}
                    onclick={let on_click = props.on_click.clone(); yew::Callback::from(move |_| on_click.emit(()))}
                >
                    {props.text.clone()}
                </button>
            }
        }
        ClickType::BggLink => {
            html! {
                <a
                    href={format!("https://boardgamegeek.com/boardgame/{}", props.text.to_lowercase().replace(" ", "-"))}
                    target="_blank"
                    rel="noopener noreferrer"
                    class={final_classes}
                >
                    {props.text.clone()}
                    <span class="ml-1 text-xs">{"🔗"}</span>
                </a>
            }
        }
        ClickType::ExternalLink => {
            html! {
                <a
                    href={props.text.clone()}
                    target="_blank"
                    rel="noopener noreferrer"
                    class={final_classes}
                >
                    {props.text.clone()}
                    <span class="ml-1 text-xs">{"↗"}</span>
                </a>
            }
        }
        ClickType::InternalLink => {
            html! {
                <a
                    href={props.text.clone()}
                    class={final_classes}
                >
                    {props.text.clone()}
                </a>
            }
        }
        ClickType::CustomAction => {
            html! {
                <button
                    class={final_classes}
                    onclick={let on_click = props.on_click.clone(); yew::Callback::from(move |_| on_click.emit(()))}
                >
                    {props.text.clone()}
                </button>
            }
        }
    }
}
