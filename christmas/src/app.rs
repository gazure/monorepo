use dioxus::prelude::*;

use crate::{
    components::{ExchangeSection, ExclusionsSection, LettersSection, ParticipantsSection},
    server::{
        Exchange, Exclusion, Participant, list_exchanges, list_excluded_letters, list_exclusions, list_participants,
    },
};

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    let mut participants = use_signal(Vec::<Participant>::new);
    let mut exclusions = use_signal(Vec::<Exclusion>::new);
    let mut excluded_letters = use_signal(Vec::<char>::new);
    let mut exchanges = use_signal(Vec::<Exchange>::new);

    let reload_participants = move || {
        spawn(async move {
            if let Ok(p) = list_participants().await {
                participants.set(p);
            }
        });
    };

    let reload_exclusions = move || {
        spawn(async move {
            if let Ok(e) = list_exclusions().await {
                exclusions.set(e);
            }
        });
    };

    let reload_letters = move || {
        spawn(async move {
            if let Ok(l) = list_excluded_letters().await {
                excluded_letters.set(l);
            }
        });
    };

    let reload_exchanges = move || {
        spawn(async move {
            if let Ok(e) = list_exchanges().await {
                exchanges.set(e);
            }
        });
    };

    let reload_all = move || {
        reload_participants();
        reload_exclusions();
        reload_letters();
        reload_exchanges();
    };

    // Initial load
    use_effect(move || {
        reload_all();
    });

    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div { class: "min-h-screen bg-gradient-to-br from-red-50 to-green-50",
            // Header
            header { class: "bg-white shadow-sm",
                div { class: "max-w-4xl mx-auto px-4 py-6",
                    h1 { class: "text-3xl font-bold text-gray-800",
                        "Christmas Gift Exchange"
                    }
                    p { class: "text-gray-600 mt-1",
                        "Manage participants, exclusions, and run your gift exchange"
                    }
                }
            }

            // Main content
            main { class: "max-w-4xl mx-auto px-4 py-8 space-y-6",
                ParticipantsSection {
                    participants,
                    on_change: move |_| {
                        reload_participants();
                        reload_exclusions();
                    }
                }

                ExclusionsSection {
                    participants,
                    exclusions,
                    on_change: move |_| reload_exclusions()
                }

                LettersSection {
                    excluded_letters,
                    on_change: move |_| reload_letters()
                }

                ExchangeSection {
                    participants,
                    exchanges,
                    on_change: move |_| reload_exchanges()
                }
            }

            // Footer
            footer { class: "text-center py-6 text-gray-500 text-sm",
                "Happy Holidays!"
            }
        }
    }
}
