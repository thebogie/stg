use yew::prelude::*;

#[function_component(Footer)]
pub fn footer() -> Html {
    html! {
        <footer class="bg-gradient-to-r from-slate-800 to-blue-600 text-white mt-auto">
            <div class="container mx-auto px-4 sm:px-6 lg:px-8 py-8 sm:py-12">
                <div class="grid grid-cols-1 md:grid-cols-3 gap-8 sm:gap-12">
                    // Brand Section
                    <div class="text-center md:text-left">
                        <div class="flex items-center justify-center md:justify-start mb-4">
                            <span class="text-2xl sm:text-3xl font-bold tracking-tight mr-2">{"STG"}</span>
                        </div>
                        <p class="text-blue-100 text-sm sm:text-base leading-relaxed max-w-md mx-auto md:mx-0">
                            {"Professional tournament management made simple. Create, organize, and manage contests with ease."}
                        </p>
                    </div>

                    // Quick Links
                    <div class="text-center md:text-left">
                        <h3 class="text-lg sm:text-xl font-semibold mb-4 sm:mb-6">{"Quick Links"}</h3>
                        <ul class="space-y-2 sm:space-y-3">
                            <li>
                                <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-sm sm:text-base flex items-center justify-center md:justify-start">
                                    <span class="mr-2">{"🏠"}</span>
                                    {"Home"}
                                </a>
                            </li>
                            <li>
                                <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-sm sm:text-base flex items-center justify-center md:justify-start">
                                    <span class="mr-2">{"🏆"}</span>
                                    {"Contests"}
                                </a>
                            </li>
                            <li>
                                <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-sm sm:text-base flex items-center justify-center md:justify-start">
                                    <span class="mr-2">{"📍"}</span>
                                    {"Venues"}
                                </a>
                            </li>
                            <li>
                                <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-sm sm:text-base flex items-center justify-center md:justify-start">
                                    <span class="mr-2">{"🎮"}</span>
                                    {"Games"}
                                </a>
                            </li>
                        </ul>
                    </div>

                    // Contact & Support
                    <div class="text-center md:text-left">
                        <h3 class="text-lg sm:text-xl font-semibold mb-4 sm:mb-6">{"Support"}</h3>
                        <ul class="space-y-2 sm:space-y-3">
                            <li>
                                <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-sm sm:text-base flex items-center justify-center md:justify-start">
                                    <span class="mr-2">{"📧"}</span>
                                    {"Contact Us"}
                                </a>
                            </li>
                            <li>
                                <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-sm sm:text-base flex items-center justify-center md:justify-start">
                                    <span class="mr-2">{"❓"}</span>
                                    {"Help Center"}
                                </a>
                            </li>
                            <li>
                                <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-sm sm:text-base flex items-center justify-center md:justify-start">
                                    <span class="mr-2">{"📄"}</span>
                                    {"Privacy Policy"}
                                </a>
                            </li>
                            <li>
                                <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-sm sm:text-base flex items-center justify-center md:justify-start">
                                    <span class="mr-2">{"📋"}</span>
                                    {"Terms of Service"}
                                </a>
                            </li>
                        </ul>
                    </div>
                </div>

                // Bottom Section
                <div class="border-t border-white/10 mt-8 sm:mt-12 pt-6 sm:pt-8">
                    <div class="flex flex-col sm:flex-row justify-between items-center space-y-4 sm:space-y-0">
                        <div class="text-center sm:text-left">
                            <p class="text-blue-100 text-sm">
                                {"© 2024 STG. All rights reserved."}
                            </p>
                            <div class="mt-2 text-xs text-blue-200 font-mono">
                                <div>{"Version: "}{crate::version::Version::current()}</div>
                                <div>{"Name: "}{crate::version::Version::name()}</div>
                                <div>{"Build Info: "}{crate::version::Version::build_info()}</div>
                            </div>
                        </div>
                        <div class="flex space-x-4 sm:space-x-6">
                            <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-lg sm:text-xl">
                                {"📱"}
                            </a>
                            <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-lg sm:text-xl">
                                {"🐦"}
                            </a>
                            <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-lg sm:text-xl">
                                {"💼"}
                            </a>
                            <a href="#" class="text-blue-100 hover:text-white transition-colors duration-200 text-lg sm:text-xl">
                                {"📷"}
                            </a>
                        </div>
                    </div>
                </div>
            </div>
        </footer>
    }
} 