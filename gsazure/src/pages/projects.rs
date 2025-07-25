use dioxus::prelude::*;

/// Projects page
#[component]
pub fn Projects() -> Element {
    rsx! {
        div { class: "min-h-screen py-20 px-4",

            // Decorative background elements
            div { class: "floating-orb w-80 h-80 top-10 right-10" }
            div { class: "floating-orb w-64 h-64 bottom-20 left-20" }
            div { class: "floating-orb w-72 h-72 top-1/2 left-1/3" }

            h1 { class: "text-5xl sm:text-6xl font-bold text-center mb-6 bg-gradient-to-r from-azure-600 to-ocean-deep bg-clip-text text-transparent",
                "Projects"
            }

            p { class: "text-lg text-azure-700 text-center mb-12 max-w-2xl mx-auto",
                "Here are some of the projects I've worked on."
            }

            div { class: "max-w-7xl mx-auto grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8",

                // Project 1
                div { class: "group relative",
                    div { class: "glass-morphism rounded-xl p-6 h-full hover:bg-white/20 transition-all duration-300 transform hover:-translate-y-2 hover:shadow-2xl",

                        div { class: "h-2 bg-azure-gradient rounded-full mb-6" }

                        h3 { class: "text-2xl font-bold text-azure-800 mb-4", "ArenaBuddy" }

                        p { class: "text-azure-700 mb-6", "A companion app for MTG Arena." }

                        div { class: "flex flex-wrap gap-2 mb-6",
                            span { class: "px-3 py-1 bg-azure-100 text-azure-700 rounded-full text-sm font-medium",
                                "Rust"
                            }
                            span { class: "px-3 py-1 bg-azure-100 text-azure-700 rounded-full text-sm font-medium",
                                "Tauri"
                            }
                            span { class: "px-3 py-1 bg-azure-100 text-azure-700 rounded-full text-sm font-medium",
                                "Leptos"
                            }
                        }

                        a {
                            href: "https://github.com/gazure/arenabuddy",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            class: "inline-flex items-center text-azure-600 hover:text-azure-800 font-medium transition-colors",
                            "View on GitHub →"
                        }
                    }
                }

                // Project 2
                div { class: "group relative",
                    div { class: "glass-morphism rounded-xl p-6 h-full hover:bg-white/20 transition-all duration-300 transform hover:-translate-y-2 hover:shadow-2xl",

                        div { class: "h-2 bg-ocean-gradient rounded-full mb-6" }

                        h3 { class: "text-2xl font-bold text-azure-800 mb-4", "This Website!" }

                        p { class: "text-azure-700 mb-6",
                            "Learning dioxus by building a personal site with it. Under construction"
                        }

                        div { class: "flex flex-wrap gap-2 mb-6",
                            span { class: "px-3 py-1 bg-azure-100 text-azure-700 rounded-full text-sm font-medium",
                                "Rust"
                            }
                            span { class: "px-3 py-1 bg-azure-100 text-azure-700 rounded-full text-sm font-medium",
                                "Dioxus"
                            }
                            span { class: "px-3 py-1 bg-azure-100 text-azure-700 rounded-full text-sm font-medium",
                                "Docker"
                            }
                            span { class: "px-3 py-1 bg-azure-100 text-azure-700 rounded-full text-sm font-medium",
                                "Fargate (for now)"
                            }
                        }

                        a {
                            href: "https://github.com/gazure/gsazure2",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            class: "inline-flex items-center text-azure-600 hover:text-azure-800 font-medium transition-colors",
                            "View on GitHub →"
                        }
                    }
                }
            }
        }
    }
}
