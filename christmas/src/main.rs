use anyhow::Result;

mod data;
mod exchange;
mod giftexchange;
mod ui;
mod utils;

fn main() -> Result<()> {
    start::start(ui::app);
    Ok(())
}
