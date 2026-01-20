use dioxus::prelude::*;

use crate::server::{Exclusion, Participant, add_exclusion, remove_exclusion};

#[component]
pub fn ExclusionsSection(
    participants: Signal<Vec<Participant>>,
    exclusions: Signal<Vec<Exclusion>>,
    on_change: EventHandler<()>,
) -> Element {
    let mut selected_a = use_signal(|| None::<i32>);
    let mut selected_b = use_signal(|| None::<i32>);
    let mut reason = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let handle_add = move |_: Event<MouseData>| {
        let a_id = match *selected_a.read() {
            Some(id) => id,
            None => return,
        };
        let b_id = match *selected_b.read() {
            Some(id) => id,
            None => return,
        };
        if a_id == b_id {
            error.set(Some("Please select two different participants".to_string()));
            return;
        }

        let reason_val = {
            let r = reason.read().trim().to_string();
            if r.is_empty() { None } else { Some(r) }
        };

        spawn(async move {
            loading.set(true);
            error.set(None);
            match add_exclusion(a_id, b_id, reason_val).await {
                Ok(_) => {
                    selected_a.set(None);
                    selected_b.set(None);
                    reason.set(String::new());
                    on_change.call(());
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    };

    let handle_remove = move |id: i32| {
        spawn(async move {
            if let Err(e) = remove_exclusion(id).await {
                error.set(Some(e.to_string()));
            } else {
                on_change.call(());
            }
        });
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow p-6",
            h2 { class: "text-xl font-semibold text-gray-800 mb-4", "Exclusions" }
            p { class: "text-gray-600 text-sm mb-4",
                "Pairs who should not be matched (e.g., spouses, siblings)"
            }

            // Add exclusion form
            div { class: "grid grid-cols-1 md:grid-cols-4 gap-2 mb-4",
                select {
                    class: "px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500",
                    disabled: *loading.read(),
                    value: {
                        let val = *selected_a.read();
                        val.map(|id| id.to_string()).unwrap_or_default()
                    },
                    onchange: move |e: Event<FormData>| {
                        selected_a.set(e.value().parse().ok());
                    },
                    option { value: "", "Select person..." }
                    for p in participants.read().iter() {
                        option { value: "{p.id}", "{p.name}" }
                    }
                }
                select {
                    class: "px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500",
                    disabled: *loading.read(),
                    value: {
                        let val = *selected_b.read();
                        val.map(|id| id.to_string()).unwrap_or_default()
                    },
                    onchange: move |e: Event<FormData>| {
                        selected_b.set(e.value().parse().ok());
                    },
                    option { value: "", "Select person..." }
                    for p in participants.read().iter() {
                        option { value: "{p.id}", "{p.name}" }
                    }
                }
                input {
                    class: "px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500",
                    r#type: "text",
                    placeholder: "Reason (optional)",
                    value: "{reason}",
                    disabled: *loading.read(),
                    oninput: move |e| reason.set(e.value()),
                }
                button {
                    class: "px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50",
                    disabled: *loading.read() || selected_a.read().is_none() || selected_b.read().is_none(),
                    onclick: handle_add,
                    "Add Exclusion"
                }
            }

            // Error display
            if let Some(err) = error.read().as_ref() {
                div { class: "text-red-600 text-sm mb-4", "{err}" }
            }

            // Exclusion list
            div { class: "space-y-2",
                if exclusions.read().is_empty() {
                    p { class: "text-gray-500 italic", "No exclusions set." }
                } else {
                    for exclusion in exclusions.read().iter() {
                        div {
                            key: "{exclusion.id}",
                            class: "flex items-center justify-between p-2 bg-gray-50 rounded",
                            span { class: "text-gray-800",
                                "{exclusion.participant_a.name} <-> {exclusion.participant_b.name}"
                                if let Some(ref r) = exclusion.reason {
                                    span { class: "text-gray-500 ml-2", "({r})" }
                                }
                            }
                            button {
                                class: "text-red-600 hover:text-red-800 text-sm",
                                onclick: {
                                    let id = exclusion.id;
                                    move |_| handle_remove(id)
                                },
                                "Remove"
                            }
                        }
                    }
                }
            }
        }
    }
}
