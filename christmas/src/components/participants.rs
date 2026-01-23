use dioxus::prelude::*;

use crate::server::{Participant, add_participant, remove_participant};

#[component]
pub fn ParticipantsSection(participants: Signal<Vec<Participant>>, on_change: EventHandler<()>) -> Element {
    let mut new_name = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let handle_add = move |_: Event<MouseData>| {
        let name = new_name.read().clone();
        if name.trim().is_empty() {
            return;
        }
        spawn(async move {
            loading.set(true);
            error.set(None);
            match add_participant(name).await {
                Ok(_) => {
                    new_name.set(String::new());
                    on_change.call(());
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    };

    let handle_remove = move |id: i32| {
        spawn(async move {
            if let Err(e) = remove_participant(id).await {
                error.set(Some(e.to_string()));
            } else {
                on_change.call(());
            }
        });
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6",
            h2 { class: "text-xl font-semibold text-gray-800 mb-4", "Participants" }

            // Add participant form
            div { class: "flex gap-2 mb-4",
                input {
                    class: "flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500",
                    r#type: "text",
                    placeholder: "Enter name...",
                    value: "{new_name}",
                    disabled: *loading.read(),
                    oninput: move |e| new_name.set(e.value()),
                    onkeypress: move |e: Event<KeyboardData>| {
                        if e.key() == Key::Enter {
                            let name = new_name.read().clone();
                            if !name.trim().is_empty() {
                                spawn(async move {
                                    loading.set(true);
                                    error.set(None);
                                    match add_participant(name).await {
                                        Ok(_) => {
                                            new_name.set(String::new());
                                            on_change.call(());
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                    loading.set(false);
                                });
                            }
                        }
                    }
                }
                button {
                    class: "px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50",
                    disabled: *loading.read() || new_name.read().trim().is_empty(),
                    onclick: handle_add,
                    "Add"
                }
            }

            // Error display
            if let Some(err) = error.read().as_ref() {
                div { class: "text-red-600 text-sm mb-4", "{err}" }
            }

            // Participant list
            div { class: "space-y-2",
                if participants.read().is_empty() {
                    p { class: "text-gray-500 italic", "No participants yet. Add some above!" }
                } else {
                    for participant in participants.read().iter() {
                        div {
                            key: "{participant.id}",
                            class: "flex items-center justify-between p-2 bg-gray-50 rounded",
                            span { class: "text-gray-800", "{participant.name}" }
                            button {
                                class: "text-red-600 hover:text-red-800 text-sm",
                                onclick: {
                                    let id = participant.id;
                                    move |_| handle_remove(id)
                                },
                                "Remove"
                            }
                        }
                    }
                }
            }

            // Count
            p { class: "text-sm text-gray-500 mt-4",
                "Total: {participants.read().len()} participant(s)"
            }
        }
    }
}
