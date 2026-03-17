use crate::auth::AuthContext;
use crate::components::auth::login_modal::LoginModal;
use crate::Route;
use gloo::events::EventListener;
use gloo_utils::window;
use wasm_bindgen::JsCast;
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component(Nav)]
pub fn nav() -> Html {
    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let show_login_modal = use_state(|| false);
    let navigator = use_navigator().unwrap();
    let current_route = use_route::<Route>().unwrap_or(Route::Home);
    let is_palette_open = use_state(|| false);

    let do_login = {
        let show_login_modal = show_login_modal.clone();
        Callback::from(move |_: ()| {
            show_login_modal.set(true);
        })
    };

    let do_logout = {
        let auth = auth.clone();
        let navigator = navigator.clone();
        Callback::from(move |_: ()| {
            auth.logout.emit(());
            navigator.push(&Route::Home);
        })
    };

    let on_modal_close = {
        let show_login_modal = show_login_modal.clone();
        Callback::from(move |_: MouseEvent| {
            show_login_modal.set(false);
        })
    };

    let toggle_mobile_menu = {
        let is_palette_open = is_palette_open.clone();
        Callback::from(move |e: MouseEvent| {
            // Prevent the opening click from also triggering the backdrop close
            // on the same pointer interaction.
            e.stop_propagation();
            is_palette_open.set(!*is_palette_open);
        })
    };

    // Navigate to create contest
    let do_create_contest = {
        let navigator = navigator.clone();
        let is_palette_open = is_palette_open.clone();
        Callback::from(move |_: ()| {
            navigator.push(&Route::Contest);
            is_palette_open.set(false);
        })
    };

    let close_palette = {
        let is_palette_open = is_palette_open.clone();
        Callback::from(move |_: MouseEvent| {
            is_palette_open.set(false);
        })
    };

    let close_palette_unit = {
        let is_palette_open = is_palette_open.clone();
        Callback::from(move |_: ()| {
            is_palette_open.set(false);
        })
    };

    let go_to_route = {
        let navigator = navigator.clone();
        let close_palette_unit = close_palette_unit.clone();
        Callback::from(move |route: Route| {
            navigator.push(&route);
            close_palette_unit.emit(());
        })
    };

    // Close on Escape while the popover is open.
    {
        let is_palette_open = is_palette_open.clone();
        use_effect_with(is_palette_open.clone(), move |open| {
            if !**open {
                return Box::new(|| {}) as Box<dyn FnOnce()>;
            }

            let is_palette_open = is_palette_open.clone();
            let listener = EventListener::new(&window(), "keydown", move |event: &web_sys::Event| {
                if let Some(e) = event.dyn_ref::<web_sys::KeyboardEvent>() {
                    if e.key() == "Escape" {
                        is_palette_open.set(false);
                    }
                }
            });

            Box::new(move || drop(listener)) as Box<dyn FnOnce()>
        });
    }

    html! {
        <>
            <nav class={classes!(
                "sticky", "top-0", "z-50", "bg-gradient-to-r", "from-slate-800", "to-blue-600",
                "text-white", "shadow-lg", "backdrop-blur-sm"
            )}>
                <div class={classes!("max-w-7xl", "mx-auto", "px-4", "sm:px-6", "lg:px-8")}>
                    <div class={classes!("flex", "justify-between", "h-16", "items-center")}>
                        // Left side - Logo and main nav
                        <div class={classes!("flex", "items-center", "space-x-4", "sm:space-x-8")}>
                            <Link<Route> to={Route::Home} classes={classes!(
                                "flex", "items-baseline", "space-x-1", "hover:transform",
                                "hover:-translate-y-0.5", "transition-transform", "duration-200",
                                "active:scale-95" // Better touch feedback
                            )}>

                                <span class={classes!("text-lg", "sm:text-xl", "font-medium", "bg-white", "text-blue-600", "px-2", "py-0.5", "rounded")}>{"STG"}</span>
                            </Link<Route>>

                            // Desktop navigation - keep compact layout until lg
                            <div class={classes!("hidden", "lg:flex", "space-x-6")}>
                                <Link<Route>
                                    to={Route::Leaderboards}
                                    classes={classes!(
                                        "px-3", "py-2", "rounded-md", "text-sm", "font-medium",
                                        "transition-colors", "duration-200", "min-h-[44px]", "flex", "items-center",
                                        if current_route == Route::Leaderboards {
                                            classes!("bg-white/20", "text-white")
                                        } else {
                                            classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                        }
                                    )}
                                >
                                    {"Leaderboards"}
                                </Link<Route>>
                                if let Some(_) = &auth.state.player {
                                    <Link<Route>
                                        to={Route::Profile}
                                        classes={classes!(
                                            "px-3", "py-2", "rounded-md", "text-sm", "font-medium",
                                            "transition-colors", "duration-200", "min-h-[44px]", "flex", "items-center", // Better touch target
                                            if current_route == Route::Profile {
                                                classes!("bg-white/20", "text-white")
                                            } else {
                                                classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                            }
                                        )}
                                    >
                                        {"Profile"}
                                    </Link<Route>>
                                    <Link<Route>
                                        to={Route::Contests}
                                        classes={classes!(
                                            "px-3", "py-2", "rounded-md", "text-sm", "font-medium",
                                            "transition-colors", "duration-200", "min-h-[44px]", "flex", "items-center",
                                            if current_route == Route::Contests {
                                                classes!("bg-white/20", "text-white")
                                            } else {
                                                classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                            }
                                        )}
                                    >
                                        {"Contests"}
                                    </Link<Route>>
                                    <Link<Route>
                                        to={Route::Venues}
                                        classes={classes!(
                                            "px-3", "py-2", "rounded-md", "text-sm", "font-medium",
                                            "transition-colors", "duration-200", "min-h-[44px]", "flex", "items-center",
                                            if current_route == Route::Venues {
                                                classes!("bg-white/20", "text-white")
                                            } else {
                                                classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                            }
                                        )}
                                    >
                                        {"Venues"}
                                    </Link<Route>>
                                    <Link<Route>
                                        to={Route::Games}
                                        classes={classes!(
                                            "px-3", "py-2", "rounded-md", "text-sm", "font-medium",
                                            "transition-colors", "duration-200", "min-h-[44px]", "flex", "items-center",
                                            if current_route == Route::Games {
                                                classes!("bg-white/20", "text-white")
                                            } else {
                                                classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                            }
                                        )}
                                    >
                                        {"Games"}
                                    </Link<Route>>
                                    <Link<Route>
                                        to={Route::Analytics}
                                        classes={classes!(
                                            "px-3", "py-2", "rounded-md", "text-sm", "font-medium",
                                            "transition-colors", "duration-200", "min-h-[44px]", "flex", "items-center",
                                            if current_route == Route::Analytics {
                                                classes!("bg-white/20", "text-white")
                                            } else {
                                                classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                            }
                                        )}
                                    >
                                        {"Statistics"}
                                    </Link<Route>>
                                    if auth.state.is_admin() {
                                        <Link<Route>
                                            to={Route::Admin}
                                            classes={classes!(
                                                "px-3", "py-2", "rounded-md", "text-sm", "font-medium",
                                                "transition-colors", "duration-200", "min-h-[44px]", "flex", "items-center",
                                                if current_route == Route::Admin {
                                                    classes!("bg-white/20", "text-white")
                                                } else {
                                                    classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                                }
                                            )}
                                        >
                                            {"👑 Admin"}
                                        </Link<Route>>
                                    }
                                }
                            </div>
                        </div>

                        // Right side - Auth buttons
                        <div class={classes!("flex", "items-center", "space-x-2", "sm:space-x-4")}>
                            if let Some(player) = &auth.state.player {
                                <div class={classes!("hidden", "lg:flex", "items-center", "space-x-6", "ml-auto", "mr-4")}>
                                    <span class={classes!("text-sm", "text-white/90")}>
                                        {"Welcome, "}
                                        <span class={classes!("font-medium", "text-white")}>{&player.email}</span>
                                        if auth.state.is_admin() {
                                            <span class={classes!("ml-2", "inline-flex", "items-center", "px-2", "py-1", "text-xs", "font-medium", "bg-yellow-400", "text-yellow-900", "rounded-full", "shadow-sm")}>
                                                <span class={classes!("mr-1")}>{"👑"}</span>
                                                {"Admin"}
                                            </span>
                                        }
                                    </span>
                                    <button
                                        onclick={do_create_contest.clone().reform(|_| ())}
                                        class={classes!(
                                            "inline-flex", "items-center", "justify-center", "px-3", "py-2",
                                            "rounded-md", "text-sm", "font-medium", "text-white",
                                            "bg-gradient-to-r", "from-blue-500", "to-indigo-600",
                                            "shadow-md", "hover:shadow-lg", "hover:brightness-105",
                                            "transition-all", "duration-200", "active:scale-95",
                                            "min-h-[36px]"
                                        )}
                                        aria-label="Create contest"
                                        title="Create contest"
                                    >
                                        <span class={classes!("mr-2")}>{"➕"}</span>
                                        <span>{"Create"}</span>
                                    </button>
                                    <button
                                        onclick={do_logout.clone().reform(|_| ())}
                                        class={classes!(
                                            "inline-flex", "items-center", "px-3", "py-1.5", "border",
                                            "border-transparent", "text-xs", "font-medium", "rounded-md",
                                            "text-blue-600", "bg-white", "hover:bg-blue-50", "focus:outline-none",
                                            "focus:ring-2", "focus:ring-offset-2", "focus:ring-blue-500",
                                            "transition-colors", "duration-200", "min-h-[32px]", "active:scale-95"
                                        )}
                                    >
                                        <span class={classes!("mr-1")}>{"↪"}</span>
                                        {"Logout"}
                                    </button>
                                </div>
                            } else {
                                <button
                                    onclick={do_login.clone().reform(|_| ())}
                                    class={classes!(
                                        "inline-flex", "items-center", "px-3", "sm:px-4", "py-2", "border",
                                        "border-transparent", "text-sm", "font-medium", "rounded-md",
                                        "text-white", "bg-blue-500", "hover:bg-blue-600", "focus:outline-none",
                                        "focus:ring-2", "focus:ring-offset-2", "focus:ring-blue-500",
                                        "transition-colors", "duration-200", "min-h-[44px]", "active:scale-95"
                                    )}
                                >
                                    <span class={classes!("mr-1", "sm:mr-2")}>{"🔑"}</span>
                                    <span class={classes!("hidden", "sm:inline")}>{"Login"}</span>
                                    <span class={classes!("sm:hidden")}>{"Sign In"}</span>
                                </button>
                            }

                            // Mobile menu button - improved touch target
                            <button
                                onclick={toggle_mobile_menu}
                                class={classes!(
                                    "lg:hidden", "inline-flex", "items-center", "justify-center", "p-3",
                                    "rounded-md", "text-white", "hover:bg-white/10", "focus:outline-none",
                                    "focus:ring-2", "focus:ring-inset", "focus:ring-white", "min-h-[44px]", "min-w-[44px]",
                                    "active:scale-95", "transition-transform", "duration-150"
                                )}
                                aria-label="Open menu"
                                aria-expanded={(*is_palette_open).to_string()}
                            >
                                <div class={classes!(
                                    "w-6", "h-6", "flex", "flex-col", "justify-center", "items-center",
                                    if *is_palette_open { classes!("space-y-0") } else { classes!("space-y-1.5") }
                                )}>
                                    <span class={classes!(
                                        "block", "w-6", "h-0.5", "bg-white", "transform",
                                        "transition-all", "duration-300", "origin-center",
                                        if *is_palette_open { classes!("rotate-45", "translate-y-0.5") } else { classes!() }
                                    )}></span>
                                    <span class={classes!(
                                        "block", "w-6", "h-0.5", "bg-white", "transition-all", "duration-300",
                                        if *is_palette_open { classes!("opacity-0") } else { classes!() }
                                    )}></span>
                                    <span class={classes!(
                                        "block", "w-6", "h-0.5", "bg-white", "transform",
                                        "transition-all", "duration-300", "origin-center",
                                        if *is_palette_open { classes!("-rotate-45", "-translate-y-0.5") } else { classes!() }
                                    )}></span>
                                </div>
                            </button>
                        </div>
                    </div>
                </div>

            </nav>

            // Anchored popover menu (Option C)
            {if *is_palette_open {
                html! {
                <div class={classes!("lg:hidden", "fixed", "inset-0", "z-[60]")}>
                    <div
                        class={classes!("absolute", "inset-0")}
                        onclick={close_palette.clone()}
                    />

                    <div
                        class={classes!(
                            "absolute", "top-16", "right-4",
                            "w-[min(18rem,calc(100vw-2rem))]",
                            "rounded-2xl",
                            "border", "border-white/15",
                            "bg-slate-900/65",
                            "shadow-2xl",
                            "backdrop-blur-xl",
                            "overflow-hidden",
                            "transform", "transition-all", "duration-200", "origin-top-right",
                            "scale-100", "opacity-100", "translate-y-0"
                        )}
                        role="dialog"
                        aria-label="Menu"
                    >
                    <div class="px-4 py-3 border-b border-white/10 flex items-center justify-between gap-3">
                        <div class="min-w-0">
                            <div class="text-white/90 text-sm font-semibold">{"Menu"}</div>
                            if let Some(player) = &auth.state.player {
                                <div class="text-white/50 text-xs truncate">{&player.email}</div>
                            }
                        </div>
                        <button
                            class="inline-flex items-center justify-center rounded-xl p-2 text-white/70 hover:text-white hover:bg-white/10 active:scale-95 transition"
                            onclick={close_palette.clone()}
                            aria-label="Close menu"
                        >
                            {"✕"}
                        </button>
                    </div>

                    <div class="py-2 px-2">
                        <button
                            type="button"
                            onclick={go_to_route.clone().reform(|_| Route::Leaderboards)}
                            class="w-full flex items-center gap-3 px-3 py-3 text-left text-base font-semibold text-white rounded-xl bg-white/10 hover:bg-white/15 active:bg-white/20 active:scale-[0.99] transition"
                        >
                            <span class="text-lg">{"🏆"}</span>
                            <span class="flex-1">
                                <span class="text-white">{"Leaderboards"}</span>
                            </span>
                        </button>

                        if auth.state.player.is_some() {
                            <button
                                type="button"
                                onclick={go_to_route.clone().reform(|_| Route::Profile)}
                                class="w-full flex items-center gap-3 px-3 py-3 text-left text-base font-semibold text-white rounded-xl bg-white/10 hover:bg-white/15 active:bg-white/20 active:scale-[0.99] transition"
                            >
                                <span class="text-lg">{"👤"}</span>
                                <span class="flex-1">
                                    <span class="text-white">{"Profile"}</span>
                                </span>
                            </button>
                            <button
                                type="button"
                                onclick={go_to_route.clone().reform(|_| Route::Contests)}
                                class="w-full flex items-center gap-3 px-3 py-3 text-left text-base font-semibold text-white rounded-xl bg-white/10 hover:bg-white/15 active:bg-white/20 active:scale-[0.99] transition"
                            >
                                <span class="text-lg">{"🏆"}</span>
                                <span class="flex-1">
                                    <span class="text-white">{"Contests"}</span>
                                </span>
                            </button>
                            <button
                                type="button"
                                onclick={go_to_route.clone().reform(|_| Route::Venues)}
                                class="w-full flex items-center gap-3 px-3 py-3 text-left text-base font-semibold text-white rounded-xl bg-white/10 hover:bg-white/15 active:bg-white/20 active:scale-[0.99] transition"
                            >
                                <span class="text-lg">{"📍"}</span>
                                <span class="flex-1">
                                    <span class="text-white">{"Venues"}</span>
                                </span>
                            </button>
                            <button
                                type="button"
                                onclick={go_to_route.clone().reform(|_| Route::Games)}
                                class="w-full flex items-center gap-3 px-3 py-3 text-left text-base font-semibold text-white rounded-xl bg-white/10 hover:bg-white/15 active:bg-white/20 active:scale-[0.99] transition"
                            >
                                <span class="text-lg">{"🎮"}</span>
                                <span class="flex-1">
                                    <span class="text-white">{"Games"}</span>
                                </span>
                            </button>
                            <button
                                type="button"
                                onclick={go_to_route.clone().reform(|_| Route::Analytics)}
                                class="w-full flex items-center gap-3 px-3 py-3 text-left text-base font-semibold text-white rounded-xl bg-white/10 hover:bg-white/15 active:bg-white/20 active:scale-[0.99] transition"
                            >
                                <span class="text-lg">{"📊"}</span>
                                <span class="flex-1">
                                    <span class="text-white">{"Statistics"}</span>
                                </span>
                            </button>
                            if auth.state.is_admin() {
                                <button
                                    type="button"
                                    onclick={go_to_route.clone().reform(|_| Route::Admin)}
                                    class="w-full flex items-center gap-3 px-3 py-3 text-left text-base font-semibold text-white rounded-xl bg-white/10 hover:bg-white/15 active:bg-white/20 active:scale-[0.99] transition"
                                >
                                    <span class="text-lg">{"👑"}</span>
                                    <span class="flex-1">
                                        <span class="text-white">{"Admin"}</span>
                                    </span>
                                </button>
                            }

                            <div class="px-2 pt-3 pb-2 border-t border-white/10">
                                <button
                                    onclick={do_create_contest.clone().reform(|_| ())}
                                    class="w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-sm font-semibold text-white bg-gradient-to-r from-blue-500/90 to-indigo-600/90 shadow-lg hover:shadow-xl hover:brightness-110 transition-all active:scale-[0.99]"
                                >
                                    <span>{"➕"}</span>
                                    {"Create contest"}
                                </button>
                                <button
                                    onclick={{
                                        let close_palette_unit = close_palette_unit.clone();
                                        let do_logout = do_logout.clone();
                                        Callback::from(move |_| {
                                            close_palette_unit.emit(());
                                            do_logout.emit(());
                                        })
                                    }}
                                    class="mt-2 w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-sm font-semibold text-white bg-white/10 hover:bg-white/15 active:bg-white/20 shadow-lg transition-all active:scale-[0.99]"
                                >
                                    <span>{"↪"}</span>
                                    {"Logout"}
                                </button>
                            </div>
                        } else {
                            <div class="px-2 pt-3 pb-2 border-t border-white/10">
                                <button
                                    onclick={{
                                        let close_palette_unit = close_palette_unit.clone();
                                        let do_login = do_login.clone();
                                        Callback::from(move |_| {
                                            close_palette_unit.emit(());
                                            do_login.emit(());
                                        })
                                    }}
                                    class="w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-sm font-semibold text-white bg-blue-500/90 hover:bg-blue-500 shadow-lg transition-all active:scale-[0.99]"
                                >
                                    <span>{"🔑"}</span>
                                    {"Login"}
                                </button>
                            </div>
                        }
                    </div>
                </div>
                </div>
                }
            } else {
                html! {}
            }}

            <LoginModal
                show={*show_login_modal}
                on_close={on_modal_close}
            />
        </>
    }
}
