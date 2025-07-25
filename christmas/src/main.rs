use anyhow::Result;

fn main() -> Result<()> {
    start::start(christmas::App);
    Ok(())
}
