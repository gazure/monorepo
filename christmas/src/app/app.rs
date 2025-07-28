use dioxus::prelude::*;

use crate::app::routes::Route;

pub fn app() -> Element {
    rsx! {
        Router::<Route> {}
    }
}
