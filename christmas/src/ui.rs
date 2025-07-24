use dioxus::{document::Title, prelude::*};

use crate::{
    data,
    exchange::ParticipantGraph,
    giftexchange::ExchangePool,
    utils::{current_year, letter_for_pool},
};

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[derive(Clone, Debug)]
pub struct ExchangePairing {
    pub giver: String,
    pub receiver: String,
}

#[derive(Clone, Debug)]
pub struct ExchangeResult {
    pub pairings: Vec<ExchangePairing>,
    pub year_letter: char,
    pub year: i32,
}

pub fn generate_exchange_pairings(pool: ExchangePool) -> ExchangeResult {
    let participants = data::get_participants_by_pool(pool);
    let graph = ParticipantGraph::from_participants(participants);
    let exchange = graph.build_exchange();

    let pairings = exchange
        .into_iter()
        .map(|(giver, receiver)| ExchangePairing { giver, receiver })
        .collect();

    let year = current_year();
    let year_letter = letter_for_pool(pool);

    ExchangeResult {
        pairings,
        year_letter,
        year,
    }
}

pub fn app() -> Element {
    let mut selected_pool = use_signal(|| ExchangePool::IslandLife);
    let mut exchange_result = use_signal(|| generate_exchange_pairings(selected_pool()));

    let regenerate = move |_| {
        exchange_result.set(generate_exchange_pairings(selected_pool()));
    };

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Title {"Christmas Gift Exchange"}
        div {
            class: "min-h-screen bg-gradient-to-br from-red-100 to-green-100 p-12",
            div {
                class: "max-w-4xl mx-auto",
                // Header
                h1 {
                    class: "text-4xl font-bold text-center mb-8 text-green-800",
                    "🎄 Christmas Gift Exchange 🎁"
                }

                // Pool selector
                div {
                    class: "bg-white rounded-lg shadow-lg p-6 mb-6",
                    h2 {
                        class: "text-2xl font-semibold mb-4 text-gray-800",
                        "Select Exchange Pool"
                    }
                    div {
                        class: "flex gap-4 flex-wrap",
                        button {
                            class: if selected_pool() == ExchangePool::IslandLife {
                                "py-3 px-6 bg-green-600 text-white rounded-md font-medium border-0 cursor-pointer"
                            } else {
                                "py-3 px-6 bg-gray-200 text-gray-700 rounded-md font-medium border-0 cursor-pointer"
                            },
                            onclick: move |_| {
                                selected_pool.set(ExchangePool::IslandLife);
                                exchange_result.set(generate_exchange_pairings(ExchangePool::IslandLife));
                            },
                            "Island Life"
                        }
                        button {
                            class: if selected_pool() == ExchangePool::Grabergishimazureson {
                                "py-3 px-6 bg-green-600 text-white rounded-md font-medium border-0 cursor-pointer"
                            } else {
                                "py-3 px-6 bg-gray-200 text-gray-700 rounded-md font-medium border-0 cursor-pointer"
                            },
                            onclick: move |_| {
                                selected_pool.set(ExchangePool::Grabergishimazureson);
                                exchange_result.set(generate_exchange_pairings(ExchangePool::Grabergishimazureson));
                            },
                            "Grabergishimazureson"
                        }
                        button {
                            class: if selected_pool() == ExchangePool::Pets {
                                "py-3 px-6 bg-green-600 text-white rounded-md font-medium border-0 cursor-pointer"
                            } else {
                                "py-3 px-6 bg-gray-200 text-gray-700 rounded-md font-medium border-0 cursor-pointer"
                            },
                            onclick: move |_| {
                                selected_pool.set(ExchangePool::Pets);
                                exchange_result.set(generate_exchange_pairings(ExchangePool::Pets));
                            },
                            "Pets"
                        }
                    }
                    button {
                        class: "mt-4 py-2 px-4 bg-red-600 text-white rounded-md font-medium flex items-center gap-2 border-0 cursor-pointer",
                        onclick: regenerate,
                        "🔄 Regenerate Pairings"
                    }
                }

                // Year and Letter Display
                div {
                    class: "bg-white rounded-lg shadow-lg p-6 mb-6",
                    div {
                        class: "text-center",
                        h3 {
                            class: "text-xl font-semibold text-gray-800 mb-2",
                            "Letter for {exchange_result().year}"
                        }
                        div {
                            class: "text-6xl font-bold text-green-600",
                            "{exchange_result().year_letter}"
                        }
                    }
                }

                // Pairings Display
                div {
                    class: "bg-white rounded-lg shadow-lg p-6",
                    h2 {
                        class: "text-2xl font-semibold mb-4 text-gray-800",
                        "Gift Exchange Pairings"
                    }
                    div {
                        class: "flex flex-col gap-3",
                        for pairing in &exchange_result().pairings {
                            div {
                                class: "flex items-center p-3 bg-gray-50 rounded-lg",
                                div {
                                    class: "flex-1 text-lg font-medium text-gray-700",
                                    "{pairing.giver}"
                                }
                                div {
                                    class: "text-2xl text-green-600 mx-4",
                                    "→"
                                }
                                div {
                                    class: "flex-1 text-lg font-medium text-gray-700 text-right",
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
