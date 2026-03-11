use crate::version::Version;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct VersionDisplayProps {
    #[prop_or_default]
    pub show_full: bool,
    #[prop_or_default]
    pub show_build_info: bool,
    #[prop_or_default]
    pub class: Classes,
}

#[function_component(VersionDisplay)]
pub fn version_display(props: &VersionDisplayProps) -> Html {
    let version_text = if props.show_build_info {
        Version::build_info()
    } else if props.show_full {
        Version::full()
    } else {
        Version::short()
    };

    html! {
        <div class={classes!(
            "text-xs", "text-white/90", "font-mono", "select-none",
            "hover:text-white", "transition-colors", "duration-200",
            "px-2", "py-1", "rounded", "bg-white/10", "hover:bg-white/20",
            "flex", "items-center", "space-x-1",
            props.class.clone()
        )}>
            <span class={classes!("text-xs")}>{"🔢"}</span>
            <span>{version_text}</span>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct VersionTooltipProps {
    #[prop_or_default]
    pub class: Classes,
}

#[function_component(VersionTooltip)]
pub fn version_tooltip(props: &VersionTooltipProps) -> Html {
    let show_tooltip = use_state(|| false);

    let on_mouse_enter = {
        let show_tooltip = show_tooltip.clone();
        Callback::from(move |_| {
            show_tooltip.set(true);
        })
    };

    let on_mouse_leave = {
        let show_tooltip = show_tooltip.clone();
        Callback::from(move |_| {
            show_tooltip.set(false);
        })
    };

    html! {
        <div
            class={classes!("relative", "inline-block", props.class.clone())}
            onmouseenter={on_mouse_enter}
            onmouseleave={on_mouse_leave}
        >
            <div class={classes!(
                "text-xs", "text-white/90", "font-mono", "cursor-help",
                "hover:text-white", "transition-colors", "duration-200",
                "px-2", "py-1", "rounded", "bg-white/10", "hover:bg-white/20",
                "flex", "items-center", "space-x-1"
            )}>
                <span class={classes!("text-xs")}>{"🔢"}</span>
                <span>{Version::short()}</span>
            </div>

            if *show_tooltip {
                <div class={classes!(
                    "absolute", "bottom-full", "left-1/2", "transform", "-translate-x-1/2",
                    "mb-2", "px-3", "py-2", "bg-gray-900", "text-white", "text-xs",
                    "rounded-lg", "shadow-lg", "z-50", "whitespace-nowrap",
                    "border", "border-gray-700"
                )}>
                    <div class={classes!("font-semibold", "mb-1")}>
                        {Version::name()}
                    </div>
                    <div class={classes!("text-gray-300")}>
                        {"Version: "}{Version::current()}
                    </div>
                    <div class={classes!("text-gray-300")}>
                        {"Build: "}{option_env!("BUILD_DATE").unwrap_or("unknown")}
                    </div>
                    <div class={classes!("text-gray-300")}>
                        {"Commit: "}{option_env!("GIT_COMMIT").unwrap_or("unknown")}
                    </div>
                    <div class={classes!(
                        "absolute", "top-full", "left-1/2", "transform", "-translate-x-1/2",
                        "w-0", "h-0", "border-l-4", "border-r-4", "border-t-4",
                        "border-transparent", "border-t-gray-900"
                    )}></div>
                </div>
            }
        </div>
    }
}
