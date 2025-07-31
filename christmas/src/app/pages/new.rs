use dioxus::prelude::*;

use crate::server::functions::new_exchange;

#[component]
pub fn NewExchange() -> Element {
    let mut exchange_name = use_signal(String::new);
    let mut exchange_description = use_signal(String::new);
    let mut exchange_year = use_signal(|| 2025);
    let mut is_creating = use_signal(|| false);
    let mut success_message = use_signal(|| None::<String>);
    let mut error_message = use_signal(|| None::<String>);

    let create_exchange = move |_| {
        spawn(async move {
            is_creating.set(true);
            error_message.set(None);
            success_message.set(None);
            let (name, desc, year) = {
                (
                    exchange_name.read().clone(),
                    exchange_description.read().clone(),
                    *exchange_year.read(),
                )
            };

            let result = new_exchange(name, desc, year).await;

            match result {
                Ok(_) => {
                    // Reset form
                    exchange_name.set(String::new());
                    exchange_description.set(String::new());
                    exchange_year.set(2025);
                    success_message.set(Some("Exchange created successfully!".to_string()));
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to create exchange: {e}")));
                }
            }
            is_creating.set(false);
        });
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow-lg p-6",
            h2 { class: "text-2xl font-semibold mb-4 text-gray-800", "Create New Exchange" }

            if let Some(success) = success_message.read().as_ref() {
                div { class: "mb-4 p-4 bg-green-100 border border-green-400 text-green-700 rounded-md",
                    "{success}"
                }
            }

            if let Some(error) = error_message.read().as_ref() {
                div { class: "mb-4 p-4 bg-red-100 border border-red-400 text-red-700 rounded-md",
                    "{error}"
                }
            }

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
    }
}
