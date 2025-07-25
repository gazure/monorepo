use dioxus::prelude::*;

use crate::{
    components::*,
    pages::{About, Blog, Projects},
};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/about")]
    About {},
    #[route("/projects")]
    Projects {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    rsx! {
        div { class: "min-h-screen bg-gradient-to-br from-azure-50 to-azure-100",
            // Animated background elements
            div { class: "fixed inset-0 overflow-hidden pointer-events-none",
                div { class: "floating-orb w-96 h-96 -top-48 -left-48" }
                div { class: "floating-orb w-64 h-64 top-1/2 right-0" }
                div { class: "floating-orb w-80 h-80 bottom-0 left-1/3" }
                div { class: "wave-pattern" }
            }

            // Navigation bar
            nav { class: "relative z-10 glass-morphism border-b border-azure-200/20 sticky top-0",
                div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                    div { class: "flex items-center justify-between h-16",
                        // Logo/Home link
                        Link {
                            to: Route::Home {},
                            class: "flex items-center space-x-2 text-azure-700 hover:text-azure-900 transition-colors",
                            div { class: "w-8 h-8 bg-azure-gradient rounded-lg shadow-lg" }
                            span { class: "font-bold text-xl", "GA" }
                        }

                        // Navigation links
                        div { class: "flex space-x-8",
                            NavLink { to: Route::About {}, "About" }
                            NavLink { to: Route::Projects {}, "Projects" }
                            NavLink { to: Route::Blog { id: 1 }, "Blog" }
                        }
                    }
                }
            }

            // Main content area
            div { class: "relative z-10", Outlet::<Route> {} }
        }
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        div { class: "min-h-screen flex items-center justify-center px-4",

            div { class: "relative z-10 text-center max-w-3xl mx-auto",

                GradientHeading { class: "text-6xl sm:text-7xl lg:text-8xl font-bold mb-4 text-glow",
                    "Grant Azure"
                }

                p { class: "text-2xl sm:text-3xl text-azure-700 mb-8 text-shadow-azure",
                    "Software Engineer"
                }

                // Navigation links
                nav { class: "flex flex-wrap justify-center gap-4 mb-12",
                    PrimaryButton { to: Route::About {}, "About" }
                    SecondaryButton { to: Route::Projects {}, "Projects" }
                    SecondaryButton { to: Route::Blog { id: 1 }, "Blog" }
                }

                // Social links
                div { class: "flex justify-center space-x-6",
                    SocialLink { href: "https://github.com/gazure", platform: "GitHub" }
                    SocialLink {
                        href: "https://www.linkedin.com/in/grant-azure/",
                        platform: "LinkedIn",
                    }
                    SocialLink {
                        href: "https://twitter.com/tehsbe",
                        platform: "Twitter",
                    }
                }
            }
        }
    }
}
