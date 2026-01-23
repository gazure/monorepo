use dioxus::prelude::*;

use crate::server::{Exchange, ExchangeResult, Participant, run_exchange};

#[component]
pub fn ExchangeSection(
    participants: Signal<Vec<Participant>>,
    exchanges: Signal<Vec<Exchange>>,
    on_change: EventHandler<()>,
) -> Element {
    let mut year = use_signal(|| chrono::Utc::now().format("%Y").to_string());
    let mut include_letter = use_signal(|| true);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut result = use_signal(|| None::<ExchangeResult>);

    let handle_run = move |_| {
        let year_val: i32 = match year.read().parse() {
            Ok(y) => y,
            Err(_) => {
                error.set(Some("Invalid year".to_string()));
                return;
            }
        };

        let include = *include_letter.read();

        spawn(async move {
            loading.set(true);
            error.set(None);
            result.set(None);

            match run_exchange(year_val, include).await {
                Ok(r) => {
                    result.set(Some(r));
                    on_change.call(());
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6",
            h2 { class: "text-xl font-semibold text-gray-800 mb-4", "Run Exchange" }

            // Configuration
            div { class: "flex flex-wrap gap-4 items-center mb-4",
                div { class: "flex items-center gap-2",
                    label { class: "text-gray-700", "Year:" }
                    input {
                        class: "w-24 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500",
                        r#type: "number",
                        value: "{year}",
                        disabled: *loading.read(),
                        oninput: move |e| year.set(e.value()),
                    }
                }
                label { class: "flex items-center gap-2 cursor-pointer",
                    input {
                        r#type: "checkbox",
                        class: "w-4 h-4 text-green-600",
                        checked: *include_letter.read(),
                        disabled: *loading.read(),
                        onchange: move |e| include_letter.set(e.checked()),
                    }
                    span { class: "text-gray-700", "Include random letter" }
                }
                button {
                    class: "px-6 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50 font-semibold",
                    disabled: *loading.read() || participants.read().len() < 2,
                    onclick: handle_run,
                    if *loading.read() { "Running..." } else { "Run Exchange" }
                }
            }

            if participants.read().len() < 2 {
                p { class: "text-amber-600 text-sm mb-4",
                    "Need at least 2 participants to run an exchange."
                }
            }

            // Error display
            if let Some(err) = error.read().as_ref() {
                div { class: "text-red-600 text-sm mb-4", "{err}" }
            }

            // Result display
            if let Some(ref res) = *result.read() {
                div { class: "mt-4 p-4 bg-green-50 rounded-lg border border-green-200",
                    h3 { class: "text-lg font-semibold text-green-800 mb-2",
                        "Exchange Results for {res.year}"
                    }
                    if let Some(letter) = res.letter {
                        p { class: "text-2xl font-bold text-red-600 mb-4",
                            "Letter: {letter}"
                        }
                    }
                    div { class: "space-y-1",
                        for pairing in res.pairings.iter() {
                            div {
                                key: "{pairing.giver}->{pairing.receiver}",
                                class: "text-gray-800",
                                span { class: "font-medium", "{pairing.giver}" }
                                span { class: "text-gray-500 mx-2", "→" }
                                span { "{pairing.receiver}" }
                            }
                        }
                    }
                }
            }
        }

        // Past exchanges
        div { class: "bg-white rounded-lg shadow p-6 mt-6",
            h2 { class: "text-xl font-semibold text-gray-800 mb-4", "Past Exchanges" }

            if exchanges.read().is_empty() {
                p { class: "text-gray-500 italic", "No exchanges yet." }
            } else {
                div { class: "space-y-4",
                    for exchange in exchanges.read().iter() {
                        div {
                            key: "{exchange.id}",
                            class: "p-4 bg-gray-50 rounded-lg",
                            div { class: "flex items-center gap-4 mb-2",
                                h3 { class: "text-lg font-semibold text-gray-800",
                                    "{exchange.year}"
                                }
                                if let Some(letter) = exchange.letter {
                                    span { class: "text-red-600 font-bold", "Letter: {letter}" }
                                }
                            }
                            div { class: "grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-1 text-sm",
                                for pairing in exchange.pairings.iter() {
                                    div { class: "text-gray-700",
                                        "{pairing.giver} → {pairing.receiver}"
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
