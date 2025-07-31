use dioxus::prelude::*;

use crate::{app::routes::Route, server::functions::get_exchanges};

#[component]
pub fn Exchanges() -> Element {
    let exchange_list = use_resource(|| async move { get_exchanges().await.unwrap_or_default() });

    rsx! {
        div { class: "bg-white rounded-lg shadow-lg p-6",
            h2 { class: "text-2xl font-semibold mb-4 text-gray-800", "Existing Exchanges" }
            if let Some(exchanges_data) = exchange_list.read().as_ref() {
                if exchanges_data.is_empty() {
                    div { class: "text-gray-500 text-center py-8",
                        "No exchanges found. "
                        Link { to: Route::NewExchange {},
                            class: "text-green-600 hover:underline",
                            "Create your first exchange!"
                        }
                    }
                } else {
                    div { class: "space-y-3",
                        for exchange in exchanges_data.iter() {
                            div { class: "p-4 bg-gray-50 rounded-lg border",
                                div { class: "flex justify-between items-start",
                                    div {
                                        h3 { class: "text-lg font-semibold text-gray-800",
                                            "{exchange.name}"
                                        }
                                        if let Some(letters) = &exchange.letters {
                                            div { class: "text-sm text-gray-600 mt-1",
                                                "Available letters: {letters}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "text-gray-500 text-center py-8",
                    "Loading exchanges..."
                }
            }
        }
    }
}
