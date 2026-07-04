//! CLI subcommands and their orchestration.

use std::fs;

use clap::Subcommand;

use crate::harness::{
    fetch::{self, FetchError},
    scaffold,
};

#[derive(Subcommand)]
pub enum Command {
    /// Fetch a problem by number and scaffold src/problems/pNNNN_<slug>.rs
    Fetch {
        /// Problem number (frontend id), e.g. 1 for Two Sum
        id: u32,
        /// Overwrite an existing scaffold
        #[arg(long)]
        force: bool,
        /// Re-download the cached problem index
        #[arg(long)]
        refresh_index: bool,
    },
    /// List scaffolded and implemented problems
    List,
}

pub async fn handle_command(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Fetch {
            id,
            force,
            refresh_index,
        } => fetch_problem(id, force, refresh_index).await,
        Command::List => list_problems(),
    }
}

async fn fetch_problem(id: u32, force: bool, refresh_index: bool) -> anyhow::Result<()> {
    let client = fetch::Client::new()?;
    let index = fetch::load_or_fetch_index(&client, id, refresh_index).await?;
    let entry = fetch::lookup(&index, id)?;
    if entry.is_paid_only {
        return Err(FetchError::PaidOnly(entry.question_frontend_id.clone(), entry.title.clone()).into());
    }
    let question = client.fetch_question(&entry.title_slug).await?;
    let module = scaffold::module_name(id, &question.title_slug);
    let rendered = scaffold::render_module(&question)?;
    let path = scaffold::write_module(&module, &rendered, force)?;
    scaffold::update_mod_rs(&module)?;
    scaffold::run_rustfmt(&path);

    println!("Scaffolded {id}. {} ({})", question.title, question.difficulty);
    println!("  {}", path.display());
    println!();
    println!("Next: solve it, then run `cargo test -p leetcode {module}`");
    Ok(())
}

fn list_problems() -> anyhow::Result<()> {
    let dir = scaffold::problems_dir();
    let mut rows = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(module) = name.strip_suffix(".rs") else {
            continue;
        };
        if module == "mod" {
            continue;
        }
        let content = fs::read_to_string(entry.path())?;
        let status = if content.contains("todo!(") {
            "scaffolded"
        } else {
            "implemented"
        };
        rows.push((module.to_string(), status));
    }
    rows.sort();

    if rows.is_empty() {
        println!("No problems yet. Start with `cargo run -p leetcode -- fetch 1`.");
        return Ok(());
    }
    let width = rows.iter().map(|(module, _)| module.len()).max().unwrap_or(0);
    for (module, status) in &rows {
        println!("{module:<width$}  {status}");
    }
    let implemented = rows.iter().filter(|(_, status)| *status == "implemented").count();
    println!();
    println!("{implemented} implemented / {} total (goal: 100)", rows.len());
    Ok(())
}
