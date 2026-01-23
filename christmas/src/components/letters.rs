use dioxus::prelude::*;

use crate::server::set_excluded_letters;

#[component]
pub fn LettersSection(excluded_letters: Signal<Vec<char>>, on_change: EventHandler<()>) -> Element {
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let toggle_letter = move |letter: char| {
        let mut current = excluded_letters.read().clone();
        if current.contains(&letter) {
            current.retain(|&c| c != letter);
        } else {
            current.push(letter);
            current.sort();
        }

        let letters_to_save = current.clone();
        spawn(async move {
            saving.set(true);
            error.set(None);
            match set_excluded_letters(letters_to_save).await {
                Ok(()) => on_change.call(()),
                Err(e) => error.set(Some(e.to_string())),
            }
            saving.set(false);
        });
    };

    let available_count = 26 - excluded_letters.read().len();

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6",
            h2 { class: "text-xl font-semibold text-gray-800 mb-4", "Letter Selection" }
            p { class: "text-gray-600 text-sm mb-4",
                "Click letters to exclude them from random selection. "
                "Excluded letters are shown in red."
            }

            // Error display
            if let Some(err) = error.read().as_ref() {
                div { class: "text-red-600 text-sm mb-4", "{err}" }
            }

            // Letter grid
            div { class: "grid grid-cols-13 gap-1 mb-4",
                for letter in 'A'..='Z' {
                    {
                        let is_excluded = excluded_letters.read().contains(&letter);
                        rsx! {
                            button {
                                key: "{letter}",
                                class: if is_excluded {
                                    "w-8 h-8 rounded font-bold text-white bg-red-500 hover:bg-red-600"
                                } else {
                                    "w-8 h-8 rounded font-bold text-gray-800 bg-green-100 hover:bg-green-200"
                                },
                                disabled: *saving.read(),
                                onclick: move |_| toggle_letter(letter),
                                "{letter}"
                            }
                        }
                    }
                }
            }

            // Status
            p { class: "text-sm text-gray-500",
                "{available_count} letter(s) available for selection"
            }
            if available_count == 0 {
                p { class: "text-sm text-red-600 mt-1",
                    "Warning: No letters available! Uncheck some to enable letter selection."
                }
            }
        }
    }
}
