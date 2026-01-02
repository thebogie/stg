use yew::prelude::*;
use yew_router::prelude::*;
use crate::auth::AuthContext;
use crate::Route;
use crate::analytics::events::{track_login_click, track_cta_create_contest_click};

#[function_component(Home)]
pub fn home() -> Html {
    let auth = use_context::<AuthContext>().expect("Auth context not found");
    let navigator = use_navigator().unwrap();

    let on_get_started = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            track_cta_create_contest_click("hero");
            navigator.push(&Route::Contests);
        })
    };

    let on_login = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            track_login_click("hero", false);
            navigator.push(&Route::Login);
        })
    };

    let on_view_profile = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&Route::Profile);
        })
    };

    html! {
        <div class="home-page min-h-screen bg-gradient-to-br from-blue-50 via-white to-indigo-50">
            // Hero Section
            <div class="relative overflow-hidden">
                <div class="container mx-auto px-4 sm:px-6 lg:px-8 py-12 sm:py-16 lg:py-20">
                    <div class="text-center max-w-4xl mx-auto">
                        // Main heading with responsive typography
                        <h1 class="text-3xl sm:text-4xl lg:text-5xl xl:text-6xl font-bold text-gray-900 mb-6 sm:mb-8 leading-tight">
                            <span class="bg-gradient-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent">
                                {"Welcome to STG"}
                            </span>
                        </h1>
                        
                        // Subtitle with better mobile spacing
                        <p class="text-lg sm:text-xl lg:text-2xl text-gray-600 mb-8 sm:mb-12 leading-relaxed max-w-3xl mx-auto">
                            {"Create and manage your contests with ease. "}
                            <span class="font-medium text-gray-800">{"Professional tournament management made simple."}</span>
                        </p>
                        
                        // Action buttons with mobile-optimized layout
                        if auth.state.player.is_some() {
                            <div class="flex flex-col sm:flex-row gap-4 sm:gap-6 justify-center items-center">
                                <button 
                                    onclick={on_get_started.clone()}
                                    class="w-full sm:w-auto inline-flex items-center justify-center px-8 py-4 text-lg font-semibold text-white bg-gradient-to-r from-blue-600 to-indigo-600 rounded-xl shadow-lg hover:shadow-xl transform hover:-translate-y-1 transition-all duration-200 active:scale-95 min-h-[56px]"
                                >
                                    <span class="mr-2 text-xl">{"🏆"}</span>
                                    {"Add Contest"}
                                </button>
                                <button 
                                    onclick={on_view_profile.clone()}
                                    class="w-full sm:w-auto inline-flex items-center justify-center px-8 py-4 text-lg font-semibold text-blue-600 bg-white border-2 border-blue-200 rounded-xl shadow-lg hover:shadow-xl hover:bg-blue-50 transform hover:-translate-y-1 transition-all duration-200 active:scale-95 min-h-[56px]"
                                >
                                    <span class="mr-2 text-xl">{"👤"}</span>
                                    {"View Profile"}
                                </button>
                            </div>
                        } else {
                            <div class="flex flex-col sm:flex-row gap-4 sm:gap-6 justify-center items-center">
                                <button 
                                    onclick={on_login.clone()}
                                    class="w-full sm:w-auto inline-flex items-center justify-center px-8 py-4 text-lg font-semibold text-white bg-gradient-to-r from-blue-600 to-indigo-600 rounded-xl shadow-lg hover:shadow-xl transform hover:-translate-y-1 transition-all duration-200 active:scale-95 min-h-[56px]"
                                >
                                    <span class="mr-2 text-xl">{"🔐"}</span>
                                    {"Log in"}
                                </button>
                            </div>
                        }
                    </div>
                </div>
            </div>

            // Social Proof Section
            <div class="py-10 sm:py-12 bg-white">
                <div class="container mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="text-center mb-6 sm:mb-8">
                        <p class="text-sm sm:text-base uppercase tracking-wide text-gray-500">{"Trusted by organizers at"}</p>
                    </div>
                    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-6 sm:gap-8 items-center opacity-80">
                        <div class="flex justify-center"><span class="text-gray-500 text-xl sm:text-2xl">{"Acme Sports"}</span></div>
                        <div class="flex justify-center"><span class="text-gray-500 text-xl sm:text-2xl">{"Northwind"}</span></div>
                        <div class="flex justify-center"><span class="text-gray-500 text-xl sm:text-2xl">{"Contoso"}</span></div>
                        <div class="flex justify-center"><span class="text-gray-500 text-xl sm:text-2xl">{"Globex"}</span></div>
                        <div class="flex justify-center"><span class="text-gray-500 text-xl sm:text-2xl">{"Umbrella"}</span></div>
                    </div>
                </div>
            </div>

            // Features Section
            <div class="py-12 sm:py-16 lg:py-20 bg-white">
                <div class="container mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="text-center mb-12 sm:mb-16">
                        <h2 class="text-2xl sm:text-3xl lg:text-4xl font-bold text-gray-900 mb-4">
                            {"Why Choose STG?"}
                        </h2>
                        <p class="text-lg text-gray-600 max-w-2xl mx-auto">
                            {"Everything you need to run successful tournaments and contests"}
                        </p>
                    </div>
                    
                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 sm:gap-8">
                        // Feature Card 1
                        <div class="bg-gradient-to-br from-blue-50 to-indigo-50 rounded-2xl p-6 sm:p-8 text-center hover:shadow-lg transition-all duration-200 transform hover:-translate-y-1">
                            <div class="w-16 h-16 bg-blue-100 rounded-full flex items-center justify-center mx-auto mb-4 sm:mb-6">
                                <span class="text-2xl sm:text-3xl">{"🏆"}</span>
                            </div>
                            <h3 class="text-xl sm:text-2xl font-semibold text-gray-900 mb-3">{"Easy Contest Creation"}</h3>
                            <p class="text-gray-600 leading-relaxed">
                                {"Create professional contests in minutes with our intuitive interface"}
                            </p>
                        </div>

                        // Feature Card 2
                        <div class="bg-gradient-to-br from-green-50 to-emerald-50 rounded-2xl p-6 sm:p-8 text-center hover:shadow-lg transition-all duration-200 transform hover:-translate-y-1">
                            <div class="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mx-auto mb-4 sm:mb-6">
                                <span class="text-2xl sm:text-3xl">{"📍"}</span>
                            </div>
                            <h3 class="text-xl sm:text-2xl font-semibold text-gray-900 mb-3">{"Venue Management"}</h3>
                            <p class="text-gray-600 leading-relaxed">
                                {"Organize venues and locations for your events with ease"}
                            </p>
                        </div>

                        // Feature Card 3
                        <div class="bg-gradient-to-br from-purple-50 to-violet-50 rounded-2xl p-6 sm:p-8 text-center hover:shadow-lg transition-all duration-200 transform hover:-translate-y-1">
                            <div class="w-16 h-16 bg-purple-100 rounded-full flex items-center justify-center mx-auto mb-4 sm:mb-6">
                                <span class="text-2xl sm:text-3xl">{"🎮"}</span>
                            </div>
                            <h3 class="text-xl sm:text-2xl font-semibold text-gray-900 mb-3">{"Game Integration"}</h3>
                            <p class="text-gray-600 leading-relaxed">
                                {"Seamlessly integrate games and track results in real-time"}
                            </p>
                        </div>
                    </div>
                </div>
            </div>

            // Product Preview Section
            <div class="py-12 sm:py-16 bg-gray-50">
                <div class="container mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="text-center mb-8 sm:mb-12">
                        <h2 class="text-2xl sm:text-3xl lg:text-4xl font-bold text-gray-900 mb-3">{"See it in action"}</h2>
                        <p class="text-lg text-gray-600 max-w-2xl mx-auto">{"A quick look at how organizing feels in STG"}</p>
                    </div>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-6 sm:gap-8">
                        <div class="bg-white rounded-2xl shadow-sm border border-gray-100 overflow-hidden">
                            <div class="aspect-[16/10] bg-gradient-to-br from-gray-50 to-gray-100" loading="lazy"></div>
                            <div class="p-4 sm:p-6">
                                <h3 class="text-lg font-semibold text-gray-900 mb-2">{"Create a contest"}</h3>
                                <p class="text-gray-600">{"Set up brackets, venues, and rules with guided steps."}</p>
                            </div>
                        </div>
                        <div class="bg-white rounded-2xl shadow-sm border border-gray-100 overflow-hidden">
                            <div class="aspect-[16/10] bg-gradient-to-br from-gray-50 to-gray-100" loading="lazy"></div>
                            <div class="p-4 sm:p-6">
                                <h3 class="text-lg font-semibold text-gray-900 mb-2">{"Track results live"}</h3>
                                <p class="text-gray-600">{"See standings and game history update in real-time."}</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            // Trust & Compliance Section
            <div class="py-10 sm:py-12 bg-white">
                <div class="container mx-auto px-4 sm:px-6 lg:px-8">
                    <div class="grid grid-cols-2 sm:grid-cols-4 gap-4 sm:gap-6 text-center">
                        <div class="rounded-xl border border-gray-100 p-4 sm:p-6">
                            <div class="text-2xl mb-1">{"🔒"}</div>
                            <p class="text-sm text-gray-600">{"Secure by design"}</p>
                        </div>
                        <div class="rounded-xl border border-gray-100 p-4 sm:p-6">
                            <div class="text-2xl mb-1">{"⚡"}</div>
                            <p class="text-sm text-gray-600">{"Fast & responsive"}</p>
                        </div>
                        <div class="rounded-xl border border-gray-100 p-4 sm:p-6">
                            <div class="text-2xl mb-1">{"🧩"}</div>
                            <p class="text-sm text-gray-600">{"Integrations ready"}</p>
                        </div>
                        <div class="rounded-xl border border-gray-100 p-4 sm:p-6">
                            <div class="text-2xl mb-1">{"🌙"}</div>
                            <p class="text-sm text-gray-600">{"Dark mode friendly"}</p>
                        </div>
                    </div>
                </div>
            </div>

            

            // Call to Action Section
            <div class="py-12 sm:py-16 bg-gray-50">
                <div class="container mx-auto px-4 sm:px-6 lg:px-8 text-center">
                    <h2 class="text-2xl sm:text-3xl lg:text-4xl font-bold text-gray-900 mb-4">
                        {"Ready to Get Started?"}
                    </h2>
                    <p class="text-lg text-gray-600 mb-8 max-w-2xl mx-auto">
                        {"Join thousands of organizers who trust STG for their tournament management needs"}
                    </p>
                    if auth.state.player.is_some() {
                        <button 
                            onclick={on_get_started.clone()}
                            class="inline-flex items-center justify-center px-8 py-4 text-lg font-semibold text-white bg-gradient-to-r from-blue-600 to-indigo-600 rounded-xl shadow-lg hover:shadow-xl transform hover:-translate-y-1 transition-all duration-200 active:scale-95 min-h-[56px]"
                        >
                            <span class="mr-2 text-xl">{"🚀"}</span>
                            {"Create Your First Contest"}
                        </button>
                    } else {
                        <button 
                            onclick={on_login.clone()}
                            class="inline-flex items-center justify-center px-8 py-4 text-lg font-semibold text-white bg-gradient-to-r from-blue-600 to-indigo-600 rounded-xl shadow-lg hover:shadow-xl transform hover:-translate-y-1 transition-all duration-200 active:scale-95 min-h-[56px]"
                        >
                            <span class="mr-2 text-xl">{"🔐"}</span>
                            {"Log in to get started"}
                        </button>
                    }
                </div>
            </div>
        </div>
    }
} 