use dioxus::prelude::*;

use crate::Route;

/// Blog page
#[component]
pub fn Blog(id: i32) -> Element {
    rsx! {
        div { class: "min-h-screen py-20 px-4",

            // Decorative background elements
            div { class: "floating-orb w-80 h-80 top-10 left-10" }
            div { class: "floating-orb w-64 h-64 bottom-10 right-10" }
            div { class: "floating-orb w-72 h-72 top-1/2 right-1/3" }

            div { class: "max-w-4xl mx-auto",

                // Blog header
                div { class: "mb-12",

                    h1 { class: "text-5xl sm:text-6xl font-bold mb-6 bg-gradient-to-r from-azure-600 to-ocean-deep bg-clip-text text-transparent",
                        "Blog Post #{id}"
                    }

                    div { class: "flex flex-wrap gap-4 mb-8",
                        div { class: "glass-morphism px-4 py-2 rounded-full flex items-center gap-2",
                            span { "📅" }
                            span { class: "text-azure-700 font-medium", "December 2024" }
                        }
                        div { class: "glass-morphism px-4 py-2 rounded-full flex items-center gap-2",
                            span { "⏱️" }
                            span { class: "text-azure-700 font-medium", "5 min read" }
                        }
                        div { class: "glass-morphism px-4 py-2 rounded-full flex items-center gap-2",
                            span { "🏷️" }
                            span { class: "text-azure-700 font-medium", "Technology" }
                        }
                    }

                    p { class: "text-xl text-azure-700 glass-morphism p-6 rounded-xl",
                        "In blog #{id}, we demonstrate how the Dioxus router works and how URL parameters can be passed as props to our route components. This is a powerful feature that enables dynamic content rendering based on the URL structure."
                    }
                }

                // Blog content
                div { class: "glass-morphism rounded-xl p-8 mb-12",

                    h2 { class: "text-3xl font-bold text-azure-800 mb-6",
                        "Dynamic Routing in Dioxus"
                    }

                    p { class: "text-lg text-azure-700 mb-6",
                        "Dioxus provides a powerful routing system that allows you to create dynamic routes with parameters. This blog post demonstrates how you can use route parameters to create dynamic content."
                    }

                    div { class: "bg-azure-50 rounded-lg p-6 mb-6",
                        h3 { class: "text-xl font-semibold text-azure-800 mb-4",
                            "Key Features:"
                        }
                        ul { class: "space-y-2 text-azure-700",
                            li { class: "flex items-start gap-2",
                                span { class: "text-azure-500 mt-1", "•" }
                                span { "Type-safe routing with compile-time checks" }
                            }
                            li { class: "flex items-start gap-2",
                                span { class: "text-azure-500 mt-1", "•" }
                                span { "Automatic parameter parsing and validation" }
                            }
                            li { class: "flex items-start gap-2",
                                span { class: "text-azure-500 mt-1", "•" }
                                span { "Nested route support" }
                            }
                            li { class: "flex items-start gap-2",
                                span { class: "text-azure-500 mt-1", "•" }
                                span { "Built-in navigation components" }
                            }
                        }
                    }

                    p { class: "text-lg text-azure-700",
                        "The current blog post ID is extracted from the URL and passed as a prop to this component, allowing us to render different content based on the ID."
                    }
                }

                // Navigation section
                div { class: "glass-morphism rounded-xl p-8",

                    h3 { class: "text-2xl font-bold text-azure-800 mb-6 text-center",
                        "Continue Reading"
                    }

                    div { class: "flex items-center justify-between",

                        Link {
                            to: Route::Blog { id: id - 1 },
                            class: "group flex items-center gap-2 px-6 py-3 bg-azure-gradient text-white rounded-lg shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 transition-all duration-200",
                            span { class: "group-hover:-translate-x-1 transition-transform", "←" }
                            span { "Previous Post" }
                        }

                        div { class: "text-3xl text-azure-400",
                            "✨"
                        }

                        Link {
                            to: Route::Blog { id: id + 1 },
                            class: "group flex items-center gap-2 px-6 py-3 bg-azure-gradient text-white rounded-lg shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 transition-all duration-200",
                            span { "Next Post" }
                            span { class: "group-hover:translate-x-1 transition-transform", "→" }
                        }
                    }
                }
            }
        }
    }
}
