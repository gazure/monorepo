use dioxus::prelude::*;

use crate::{backend::{load_exchanges, new_exchange, get_exchanges}, model::ExchangePool};

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

pub fn app() -> Element {
    let exchanges = use_resource(|| async move { load_exchanges().await.unwrap_or_default() });
    let mut exchange_list = use_resource(|| async move { get_exchanges().await.unwrap_or_default() });
    let mut selected_pool = use_signal(|| None::<ExchangePool>);
    let mut current_view = use_signal(|| "pools"); // "pools" or "exchanges"
    let mut exchange_result = use_signal(|| {
        if let Some(pool) = selected_pool.read().as_ref() {
            pool.generate_pairings()
        } else {
            Default::default()
        }
    });

    // Form state for new exchange
    let mut exchange_name = use_signal(|| String::new());
    let mut exchange_description = use_signal(|| String::new());
    let mut exchange_year = use_signal(|| 2024);
    let mut is_creating = use_signal(|| false);

    let regenerate = move |_| {
        if let Some(pool) = selected_pool.read().as_ref() {
            exchange_result.set(pool.generate_pairings());
        }
    };

    let create_exchange = move |_| {
        spawn(async move {
            is_creating.set(true);
            let result = new_exchange(
                exchange_name.read().clone(),
                exchange_description.read().clone(),
                *exchange_year.read(),
            ).await;

            match result {
                Ok(_) => {
                    // Reset form
                    exchange_name.set(String::new());
                    exchange_description.set(String::new());
                    exchange_year.set(2024);
                    // Refresh the exchange list
                    exchange_list.restart();
                }
                Err(_) => {
                    // Handle error - could add error state here
                }
            }
            is_creating.set(false);
        });
    };

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
                        button {
                            class: if *current_view.read() == "pools" {
                                "py-2 px-4 bg-green-600 text-white rounded-md font-medium border-0 cursor-pointer"
                            } else {
                                "py-2 px-4 bg-gray-200 text-gray-700 rounded-md font-medium border-0 cursor-pointer"
                            },
                            onclick: move |_| current_view.set("pools"),
                            "Gift Exchange Pools"
                        }
                        button {
                            class: if *current_view.read() == "exchanges" {
                                "py-2 px-4 bg-green-600 text-white rounded-md font-medium border-0 cursor-pointer"
                            } else {
                                "py-2 px-4 bg-gray-200 text-gray-700 rounded-md font-medium border-0 cursor-pointer"
                            },
                            onclick: move |_| current_view.set("exchanges"),
                            "Manage Exchanges"
                        }
                    }
                }

                if *current_view.read() == "pools" {
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

                if *current_view.read() == "exchanges" {
                    // New Exchange Form
                    div { class: "bg-white rounded-lg shadow-lg p-6 mb-6",
                        h2 { class: "text-2xl font-semibold mb-4 text-gray-800", "Create New Exchange" }
                        div { class: "space-y-4",
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-2", "Exchange Name" }
                                input {
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500",
                                    r#type: "text",
                                    placeholder: "Enter exchange name",
                                    value: "{exchange_name}",
                                    oninput: move |e| exchange_name.set(e.value())
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-2", "Description" }
                                textarea {
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500",
                                    placeholder: "Enter description",
                                    rows: "3",
                                    value: "{exchange_description}",
                                    oninput: move |e| exchange_description.set(e.value())
                                }
                            }
                            div {
                                label { class: "block text-sm font-medium text-gray-700 mb-2", "Year" }
                                input {
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500",
                                    r#type: "number",
                                    value: "{exchange_year}",
                                    oninput: move |e| {
                                        if let Ok(year) = e.value().parse::<i32>() {
                                            exchange_year.set(year);
                                        }
                                    }
                                }
                            }
                            button {
                                class: "w-full py-2 px-4 bg-green-600 text-white rounded-md font-medium border-0 cursor-pointer disabled:opacity-50",
                                disabled: *is_creating.read(),
                                onclick: create_exchange,
                                if *is_creating.read() { "Creating..." } else { "Create Exchange" }
                            }
                        }
                    }

                    // Exchanges List
                    div { class: "bg-white rounded-lg shadow-lg p-6",
                        h2 { class: "text-2xl font-semibold mb-4 text-gray-800", "Existing Exchanges" }
                        if let Some(exchanges_data) = exchange_list.read().as_ref() {
                            if exchanges_data.is_empty() {
                                div { class: "text-gray-500 text-center py-8",
                                    "No exchanges found. Create your first exchange above!"
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
        }
    }
}
