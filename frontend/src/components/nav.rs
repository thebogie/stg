use yew::prelude::*;
use yew_router::prelude::*;
use crate::auth::AuthContext;
use crate::Route;
use crate::components::auth::login_modal::LoginModal;


#[function_component(Nav)]
pub fn nav() -> Html {
    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let show_login_modal = use_state(|| false);
    let navigator = use_navigator().unwrap();
    let current_route = use_route::<Route>().unwrap_or(Route::Home);
    let is_mobile_menu_open = use_state(|| false);

    let on_login_click = {
        let show_login_modal = show_login_modal.clone();
        Callback::from(move |_| {
            show_login_modal.set(true);
        })
    };

    let on_logout_click = {
        let auth = auth.clone();
        let navigator = navigator.clone();
        Callback::from(move |_| {
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
        let is_mobile_menu_open = is_mobile_menu_open.clone();
        Callback::from(move |_| {
            is_mobile_menu_open.set(!*is_mobile_menu_open);
        })
    };

    // Navigate to create contest
    let on_create_click = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&Route::Contest);
        })
    };

    // Close mobile menu when navigating
    let close_mobile_menu = {
        let is_mobile_menu_open = is_mobile_menu_open.clone();
        Callback::from(move |_| {
            is_mobile_menu_open.set(false);
        })
    };

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
                            
                            // Desktop navigation - hidden on mobile
                            <div class={classes!("hidden", "md:flex", "space-x-6")}>
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
                                <div class={classes!("hidden", "md:flex", "items-center", "space-x-6", "ml-auto", "mr-4")}>
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
                                        onclick={on_create_click.clone()}
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
                                        onclick={on_logout_click.clone()}
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
                                    onclick={on_login_click.clone()}
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
                                    "md:hidden", "inline-flex", "items-center", "justify-center", "p-3", 
                                    "rounded-md", "text-white", "hover:bg-white/10", "focus:outline-none", 
                                    "focus:ring-2", "focus:ring-inset", "focus:ring-white", "min-h-[44px]", "min-w-[44px]",
                                    "active:scale-95", "transition-transform", "duration-150"
                                )}
                                aria-label="Toggle mobile menu"
                            >
                                <div class={classes!(
                                    "w-6", "h-6", "flex", "flex-col", "justify-center", "items-center",
                                    if *is_mobile_menu_open { classes!("space-y-0") } else { classes!("space-y-1.5") }
                                )}>
                                    <span class={classes!(
                                        "block", "w-6", "h-0.5", "bg-white", "transform", 
                                        "transition-all", "duration-300", "origin-center",
                                        if *is_mobile_menu_open { classes!("rotate-45", "translate-y-0.5") } else { classes!() }
                                    )}></span>
                                    <span class={classes!(
                                        "block", "w-6", "h-0.5", "bg-white", "transition-all", "duration-300",
                                        if *is_mobile_menu_open { classes!("opacity-0") } else { classes!() }
                                    )}></span>
                                    <span class={classes!(
                                        "block", "w-6", "h-0.5", "bg-white", "transform", 
                                        "transition-all", "duration-300", "origin-center",
                                        if *is_mobile_menu_open { classes!("-rotate-45", "-translate-y-0.5") } else { classes!() }
                                    )}></span>
                                </div>
                            </button>
                        </div>
                    </div>
                </div>

                // Enhanced mobile menu with better animations and touch targets
                <div class={classes!(
                    "md:hidden", "transition-all", "duration-300", "ease-in-out", "border-t", "border-white/10",
                    if *is_mobile_menu_open { 
                        classes!("max-h-96", "opacity-100", "visible") 
                    } else { 
                        classes!("max-h-0", "opacity-0", "invisible", "overflow-hidden") 
                    }
                )}>
                    <div class={classes!("px-4", "pt-4", "pb-6", "space-y-2", "bg-gradient-to-b", "from-slate-800/95", "to-blue-600/95", "backdrop-blur-sm")}>
                        if let Some(_) = &auth.state.player {
                            <div onclick={close_mobile_menu.clone()}>
                                <Link<Route> 
                                    to={Route::Profile} 
                                    classes={classes!(
                                        "block", "px-4", "py-3", "rounded-lg", "text-base", "font-medium", 
                                        "transition-all", "duration-200", "min-h-[48px]", "flex", "items-center",
                                        "active:scale-95", "active:bg-white/20",
                                        if current_route == Route::Profile {
                                            classes!("bg-white/20", "text-white", "shadow-lg")
                                        } else {
                                            classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                        }
                                    )}
                                >
                                    <span class={classes!("mr-3", "text-lg")}>{"👤"}</span>
                                    {"Profile"}
                                </Link<Route>>
                            </div>
                            // Mobile: Create button
                            <div class={classes!("px-4")}>
                                <button
                                    onclick={on_create_click.clone()}
                                    class={classes!(
                                        "w-full", "inline-flex", "justify-center", "items-center",
                                        "px-4", "py-3", "rounded-lg", "text-base", "font-medium",
                                        "text-white", "bg-gradient-to-r", "from-blue-500", "to-indigo-600",
                                        "shadow-lg", "hover:shadow-xl", "hover:brightness-105",
                                        "transition-all", "duration-200", "active:scale-95"
                                    )}
                                >
                                    <span class={classes!("mr-2")}>{"➕"}</span>
                                    {"Create Contest"}
                                </button>
                            </div>
                            <div onclick={close_mobile_menu.clone()}>
                                <Link<Route> 
                                    to={Route::Contests} 
                                    classes={classes!(
                                        "block", "px-4", "py-3", "rounded-lg", "text-base", "font-medium", 
                                        "transition-all", "duration-200", "min-h-[48px]", "flex", "items-center",
                                        "active:scale-95", "active:bg-white/20",
                                        if current_route == Route::Contests {
                                            classes!("bg-white/20", "text-white", "shadow-lg")
                                        } else {
                                            classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                        }
                                    )}
                                >
                                    <span class={classes!("mr-3", "text-lg")}>{"🏆"}</span>
                                    {"Contests"}
                                </Link<Route>>
                            </div>
                            <div onclick={close_mobile_menu.clone()}>
                                <Link<Route> 
                                    to={Route::Venues} 
                                    classes={classes!(
                                        "block", "px-4", "py-3", "rounded-lg", "text-base", "font-medium", 
                                        "transition-all", "duration-200", "min-h-[48px]", "flex", "items-center",
                                        "active:scale-95", "active:bg-white/20",
                                        if current_route == Route::Venues {
                                            classes!("bg-white/20", "text-white", "shadow-lg")
                                        } else {
                                            classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                        }
                                    )}
                                >
                                    <span class={classes!("mr-3", "text-lg")}>{"📍"}</span>
                                    {"Venues"}
                                </Link<Route>>
                            </div>
                            <div onclick={close_mobile_menu.clone()}>
                                <Link<Route> 
                                    to={Route::Games} 
                                    classes={classes!(
                                        "block", "px-4", "py-3", "rounded-lg", "text-base", "font-medium", 
                                        "transition-all", "duration-200", "min-h-[48px]", "flex", "items-center",
                                        "active:scale-95", "active:bg-white/20",
                                        if current_route == Route::Games {
                                            classes!("bg-white/20", "text-white", "shadow-lg")
                                        } else {
                                            classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                        }
                                    )}
                                >
                                    <span class={classes!("mr-3", "text-lg")}>{"🎮"}</span>
                                    {"Games"}
                                </Link<Route>>
                            </div>
                            <div onclick={close_mobile_menu.clone()}>
                                <Link<Route> 
                                    to={Route::Analytics} 
                                    classes={classes!(
                                        "block", "px-4", "py-3", "rounded-lg", "text-base", "font-medium", 
                                        "transition-all", "duration-200", "min-h-[48px]", "flex", "items-center",
                                        "active:scale-95", "active:bg-white/20",
                                        if current_route == Route::Analytics {
                                            classes!("bg-white/20", "text-white", "shadow-lg")
                                        } else {
                                            classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                        }
                                    )}
                                >
                                    <span class={classes!("mr-3", "text-lg")}>{"📊"}</span>
                                    {"Statistics"}
                                </Link<Route>>
                            </div>
                            if auth.state.is_admin() {
                                <div onclick={close_mobile_menu.clone()}>
                                    <Link<Route> 
                                        to={Route::Admin} 
                                        classes={classes!(
                                            "block", "px-4", "py-3", "rounded-lg", "text-base", "font-medium", 
                                            "transition-all", "duration-200", "min-h-[48px]", "flex", "items-center",
                                            "active:scale-95", "active:bg-white/20",
                                            if current_route == Route::Admin {
                                                classes!("bg-white/20", "text-white", "shadow-lg")
                                            } else {
                                                classes!("text-white/90", "hover:bg-white/10", "hover:text-white")
                                            }
                                        )}
                                    >
                                        <span class={classes!("mr-3", "text-lg")}>{"👑"}</span>
                                        {"Admin"}
                                    </Link<Route>>
                                </div>
                            }
                        }
                        
                        // User info and logout section
                        if let Some(player) = &auth.state.player {
                            <div class={classes!("mt-6", "pt-4", "border-t", "border-white/10")}>
                                <div class={classes!("px-4", "py-3", "rounded-lg", "bg-white/10", "backdrop-blur-sm")}>
                                    <span class={classes!("block", "text-sm", "text-white/90", "mb-2")}>
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
                                        onclick={on_logout_click.clone()}
                                        class={classes!(
                                            "w-full", "inline-flex", "justify-center", "items-center", 
                                            "px-3", "py-2", "border", "border-transparent", "text-xs", "font-medium", 
                                            "rounded-lg", "text-blue-600", "bg-white", "hover:bg-blue-50", 
                                            "focus:outline-none", "focus:ring-2", "focus:ring-offset-2", 
                                            "focus:ring-blue-500", "transition-all", "duration-200", "min-h-[36px]",
                                            "active:scale-95", "shadow-lg"
                                        )}
                                    >
                                        <span class={classes!("mr-1")}>{"↪"}</span>
                                        {"Logout"}
                                    </button>
                                </div>
                            </div>
                        } else {
                            <div class={classes!("mt-6", "pt-4", "border-t", "border-white/10")}>
                                <button 
                                    onclick={on_login_click.clone()}
                                    class={classes!(
                                        "w-full", "inline-flex", "justify-center", "items-center", "px-4", 
                                        "py-3", "border", "border-transparent", "text-sm", "font-medium", 
                                        "rounded-lg", "text-white", "bg-blue-500", "hover:bg-blue-600", 
                                        "focus:outline-none", "focus:ring-2", "focus:ring-offset-2", 
                                        "focus:ring-blue-500", "transition-all", "duration-200", "min-h-[48px]",
                                        "active:scale-95", "shadow-lg"
                                    )}
                                >
                                    <span class={classes!("mr-2")}>{"🔑"}</span>
                                    {"Login"}
                                </button>
                            </div>
                        }
                    </div>
                </div>
            </nav>

            <LoginModal 
                show={*show_login_modal} 
                on_close={on_modal_close}
            />
        </>
    }
} 