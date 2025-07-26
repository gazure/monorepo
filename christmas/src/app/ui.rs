use dioxus::{document::Title, prelude::*};

use crate::{backend::load_exchanges, model::ExchangePool};

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

pub fn app() -> Element {
    let exchanges = use_resource(|| async move { load_exchanges().await.unwrap_or_default() });
    let mut selected_pool = use_signal(|| None::<ExchangePool>);
    let mut exchange_result = use_signal(|| {
        if let Some(pool) = selected_pool.read().as_ref() {
            pool.generate_pairings()
        } else {
            Default::default()
        }
    });

    let regenerate = move |_| {
        if let Some(pool) = selected_pool.read().as_ref() {
            exchange_result.set(pool.generate_pairings());
        }
    };

    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        Title { "Christmas Gift Exchange" }
        div { class: "min-h-screen bg-gradient-to-br from-red-100 to-green-100 p-12",
            div { class: "max-w-4xl mx-auto",
                // Header
                h1 { class: "text-4xl font-bold text-center mb-8 text-green-800",
                    "🎄 Christmas Gift Exchange 🎁"
                }

                // Pool selector
                div { class: "bg-white rounded-lg shadow-lg p-6 mb-6",
                    h2 { class: "text-2xl font-semibold mb-4 text-gray-800", "Select Exchange Pool" }
                    div { class: "flex gap-4 flex-wrap",
                        if let Some(exchanges_data) = exchanges.read().as_ref() {
                            for pool in exchanges_data.iter() {
                                button {
                                    class: if selected_pool.read().as_ref().map(|p| &p.exchange.name) == Some(&pool.exchange.name) {
                                        "py-3 px-6 bg-green-600 text-white rounded-md font-medium border-0 cursor-pointer"
                                    } else {
                                        "py-3 px-6 bg-gray-200 text-gray-700 rounded-md font-medium border-0 cursor-pointer"
                                    },
                                    onclick: {
                                        let pool_clone = (*pool).clone();
                                        move |_| {
                                            let result = pool_clone.generate_pairings();
                                            selected_pool.set(Some(pool_clone.clone()));
                                            exchange_result.set(result);
                                        }
                                    },
                                    "{pool.exchange.name}"
                                }
                            }
                        }
                    }
                    button {
                        class: "mt-4 py-2 px-4 bg-red-600 text-white rounded-md font-medium flex items-center gap-2 border-0 cursor-pointer",
                        onclick: regenerate,
                        "🔄 Regenerate Pairings"
                    }
                }

                if selected_pool.read().is_some() {
                    // Year and Letter Display
                    div { class: "bg-white rounded-lg shadow-lg p-6 mb-6",
                        div { class: "text-center",
                            h3 { class: "text-xl font-semibold text-gray-800 mb-2",
                                "Letter for {exchange_result().year}"
                            }
                            div { class: "text-6xl font-bold text-green-600",
                                "{exchange_result().year_letter}"
                            }
                        }
                    }

                    // Pairings Display
                    div { class: "bg-white rounded-lg shadow-lg p-6",
                        h2 { class: "text-2xl font-semibold mb-4 text-gray-800", "Gift Exchange Pairings" }
                        div { class: "flex flex-col gap-3",
                            for pairing in &exchange_result().pairings {
                                div { class: "flex items-center p-3 bg-gray-50 rounded-lg",
                                    div { class: "flex-1 text-lg font-medium text-gray-700",
                                        "{pairing.giver}"
                                    }
                                    div { class: "text-2xl text-green-600 mx-4", "→" }
                                    div { class: "flex-1 text-lg font-medium text-gray-700 text-right",
                                        "{pairing.receiver}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
