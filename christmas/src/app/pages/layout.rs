use dioxus::prelude::*;

use crate::app::routes::Route;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn Layout() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        document::Title { "Christmas Gift Exchange" }
        div { class: "min-h-screen bg-gradient-to-br from-red-100 to-green-100 p-12",
            div { class: "max-w-4xl mx-auto",
                // Header
                h1 { class: "text-4xl font-bold text-center mb-8 text-green-800",
                    "🎄 Christmas Gift Exchange 🎁"
                }

                // Navigation
                div { class: "bg-white rounded-lg shadow-lg p-6 mb-6",
                    div { class: "flex gap-4",
                        Link { to: Route::Home {},
                            class: "py-2 px-4 bg-green-600 text-white rounded-md font-medium no-underline hover:bg-green-700",
                            "Gift Exchange Pools"
                        }
                        Link { to: Route::NewExchange {},
                            class: "py-2 px-4 bg-gray-200 text-gray-700 rounded-md font-medium no-underline hover:bg-gray-300",
                            "New Exchange"
                        }
                        Link { to: Route::Exchanges {},
                            class: "py-2 px-4 bg-gray-200 text-gray-700 rounded-md font-medium no-underline hover:bg-gray-300",
                            "Manage Exchanges"
                        }
                    }
                }

                Outlet::<Route> {}
            }
        }
    }
}
