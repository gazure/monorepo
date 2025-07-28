use dioxus::prelude::*;

use crate::app::pages::{exchanges::Exchanges, home::Home, layout::Layout, new::NewExchange};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(Layout)]
    #[route("/")]
    Home {},
    #[route("/new")]
    NewExchange {},
    #[route("/exchanges")]
    Exchanges {},
}
