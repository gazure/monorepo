use dioxus::{document::Title, prelude::*};

use crate::{
    data,
    exchange::ParticipantGraph,
    giftexchange::ExchangePool,
    utils::{current_year, letter_for_pool},
};

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
        Title {"Christmas Gift Exchange"}
        div {
            style: "min-height: 100vh; background: linear-gradient(to bottom right, #fee2e2, #dcfce7); padding: 2rem;",
            div {
                style: "max-width: 56rem; margin: 0 auto;",
                // Header
                h1 {
                    style: "font-size: 2.5rem; font-weight: bold; text-align: center; margin-bottom: 2rem; color: #166534;",
                    "🎄 Christmas Gift Exchange 🎁"
                }

                // Pool selector
                div {
                    style: "background: white; border-radius: 0.5rem; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1); padding: 1.5rem; margin-bottom: 1.5rem;",
                    h2 {
                        style: "font-size: 1.5rem; font-weight: 600; margin-bottom: 1rem; color: #1f2937;",
                        "Select Exchange Pool"
                    }
                    div {
                        style: "display: flex; gap: 1rem; flex-wrap: wrap;",
                        button {
                            style: if selected_pool() == ExchangePool::IslandLife {
                                "padding: 0.75rem 1.5rem; background: #16a34a; color: white; border-radius: 0.375rem; font-weight: 500; border: none; cursor: pointer;"
                            } else {
                                "padding: 0.75rem 1.5rem; background: #e5e7eb; color: #374151; border-radius: 0.375rem; font-weight: 500; border: none; cursor: pointer;"
                            },
                            onclick: move |_| {
                                selected_pool.set(ExchangePool::IslandLife);
                                exchange_result.set(generate_exchange_pairings(ExchangePool::IslandLife));
                            },
                            "Island Life"
                        }
                        button {
                            style: if selected_pool() == ExchangePool::Grabergishimazureson {
                                "padding: 0.75rem 1.5rem; background: #16a34a; color: white; border-radius: 0.375rem; font-weight: 500; border: none; cursor: pointer;"
                            } else {
                                "padding: 0.75rem 1.5rem; background: #e5e7eb; color: #374151; border-radius: 0.375rem; font-weight: 500; border: none; cursor: pointer;"
                            },
                            onclick: move |_| {
                                selected_pool.set(ExchangePool::Grabergishimazureson);
                                exchange_result.set(generate_exchange_pairings(ExchangePool::Grabergishimazureson));
                            },
                            "Grabergishimazureson"
                        }
                        button {
                            style: if selected_pool() == ExchangePool::Pets {
                                "padding: 0.75rem 1.5rem; background: #16a34a; color: white; border-radius: 0.375rem; font-weight: 500; border: none; cursor: pointer;"
                            } else {
                                "padding: 0.75rem 1.5rem; background: #e5e7eb; color: #374151; border-radius: 0.375rem; font-weight: 500; border: none; cursor: pointer;"
                            },
                            onclick: move |_| {
                                selected_pool.set(ExchangePool::Pets);
                                exchange_result.set(generate_exchange_pairings(ExchangePool::Pets));
                            },
                            "Pets"
                        }
                    }
                    button {
                        style: "margin-top: 1rem; padding: 0.5rem 1rem; background: #dc2626; color: white; border-radius: 0.375rem; font-weight: 500; display: flex; align-items: center; gap: 0.5rem; border: none; cursor: pointer;",
                        onclick: regenerate,
                        "🔄 Regenerate Pairings"
                    }
                }

                // Year and Letter Display
                div {
                    style: "background: white; border-radius: 0.5rem; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1); padding: 1.5rem; margin-bottom: 1.5rem;",
                    div {
                        style: "text-align: center;",
                        h3 {
                            style: "font-size: 1.25rem; font-weight: 600; color: #1f2937; margin-bottom: 0.5rem;",
                            "Letter for {exchange_result().year}"
                        }
                        div {
                            style: "font-size: 4rem; font-weight: bold; color: #16a34a;",
                            "{exchange_result().year_letter}"
                        }
                    }
                }

                // Pairings Display
                div {
                    style: "background: white; border-radius: 0.5rem; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1); padding: 1.5rem;",
                    h2 {
                        style: "font-size: 1.5rem; font-weight: 600; margin-bottom: 1rem; color: #1f2937;",
                        "Gift Exchange Pairings"
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 0.75rem;",
                        for pairing in &exchange_result().pairings {
                            div {
                                style: "display: flex; align-items: center; padding: 0.75rem; background: #f9fafb; border-radius: 0.5rem;",
                                div {
                                    style: "flex: 1; font-size: 1.125rem; font-weight: 500; color: #374151;",
                                    "{pairing.giver}"
                                }
                                div {
                                    style: "font-size: 1.5rem; color: #16a34a; margin: 0 1rem;",
                                    "→"
                                }
                                div {
                                    style: "flex: 1; font-size: 1.125rem; font-weight: 500; color: #374151; text-align: right;",
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
