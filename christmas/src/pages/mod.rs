mod admin;
mod history;
mod home;
mod login;
mod pool;
mod reveal;
mod year;

pub use admin::Admin;
pub use history::History;
pub use home::Home;
pub use login::Login;
pub use pool::PoolPage;
pub use reveal::{Ceremony, Reveal};
pub use year::YearPage;

/// The year the exchange is currently about.
pub fn current_year() -> i32 {
    use chrono::Datelike;
    chrono::Utc::now().year()
}
