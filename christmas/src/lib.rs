mod data;
mod exchange;
mod giftexchange;
mod ui;
mod utils;

use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    ui::app()
}
