// Not used currently. need to decide if I keep this.
use anyhow::Result;
use std::path::PathBuf;

use chrono::{Datelike, Local};
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::{ExchangePool, Participant};

pub fn init_db(path: PathBuf) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    let migrations = Migrations::new(
        vec![
            M::up("
                CREATE TABLE IF NOT EXISTS exchange (
                    id INTEGER PRIMARY KEY,
                    year INTEGER NOT NULL,
                    name TEXT NOT NULL
                )
            "),
            M::up("
                CREATE TABLE IF NOT EXISTS participant (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL
                )
            "),
            M::up("
                CREATE TABLE IF NOT EXISTS participant_exchange (
                    id INTEGER PRIMARY KEY,
                    participant_id INTEGER NOT NULL,
                    exchange_id INTEGER NOT NULL,
                    FOREIGN KEY (participant_id) REFERENCES participant(id),
                    FOREIGN KEY (exchange_id) REFERENCES exchange(id)
                )
            "),
            M::up("CREATE TABLE IF NOT EXISTS participant_exclusion (
                    id INTEGER PRIMARY KEY,
                    participant_id INTEGER NOT NULL,
                    excluded_participant_id INTEGER NOT NULL,
                    year INTEGER DEFAULT NULL,
                    FOREIGN KEY (participant_id) REFERENCES participant(id),
                    FOREIGN KEY (excluded_participant_id) REFERENCES participant(id)
                )
            "),
            M::up(
                "CREATE TABLE IF NOT EXISTS exchange_pairing (
                    id INTEGER PRIMARY KEY,
                    giver_id INTEGER NOT NULL,
                    receiver_id INTEGER NOT NULL,
                    exchange_id INTEGER NOT NULL,
                    FOREIGN KEY (giver_id) REFERENCES participant(id),
                    FOREIGN KEY (receiver_id) REFERENCES participant(id),
                    FOREIGN KEY (exchange_id) REFERENCES exchange(id)
                )"
            ),
            M::up(
                "
                CREATE UNIQUE INDEX IF NOT EXISTS idx_participant_exchange ON participant_exchange (participant_id, exchange_id);
                CREATE UNIQUE INDEX IF NOT EXISTS idx_participant_exclusion ON participant_exclusion (participant_id, excluded_participant_id);
                "
            )
        ]
    );
    migrations.to_latest(&mut conn)?;
    Ok(conn)
}

pub fn add_exchange(conn: &mut Connection, exchanges: &[ExchangePool]) -> Result<Vec<i64>> {
    let tx = conn.transaction()?;
    let year = Local::now().year();
    let mut exchange_ids = vec![];
    for exchange in exchanges {
        let exchange_exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM exchange WHERE name = ?1 and year = ?2)",
            [&exchange.to_string(), &year.to_string()],
            |row| row.get(0),
        )?;
        if !exchange_exists {
            tx.execute(
                "INSERT INTO exchange (year, name) VALUES (?1, ?2)",
                [&year.to_string(), &exchange.to_string()],
            )?;
        }
        let exchange_id: i64 = tx.query_row(
            "SELECT id FROM exchange WHERE name = ?1 and year = ?2",
            [&exchange.to_string(), &year.to_string()],
            |row| row.get(0),
        )?;
        exchange_ids.push(exchange_id);
    }
    tx.commit()?;

    Ok(exchange_ids)
}

pub fn add_participant(conn: &mut Connection, participant: &Participant) -> Result<i64> {
    let year = Local::now().year();
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM participant WHERE name = ?1)",
        [&participant.name],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute(
            "INSERT INTO participant (name) VALUES (?1)",
            [&participant.name],
        )?;
    }
    let partipant_id: i64 = conn.query_row(
        "SELECT id FROM participant WHERE name = ?1",
        [&participant.name],
        |row| row.get(0),
    )?;

    for exchange in &participant.exchange_pools {
        let exchange_id: i64 = conn.query_row(
            "SELECT id FROM exchange WHERE name = ?1 and year = ?2",
            (&exchange.to_string(), &year),
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO participant_exchange (participant_id, exchange_id) VALUES (?1, ?2)",
            [&partipant_id, &exchange_id],
        )?;
    }

    for exclusion in &participant.exclusions {
        let excluded_participant_id: i64 = conn.query_row(
            "SELECT id FROM participant WHERE name = ?1",
            [&exclusion],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO participant_exclusion (participant_id, excluded_participant_id, year) VALUES (?1, ?2, ?3)",
            (&partipant_id, &excluded_participant_id, &year),
        )?;
    }

    // let mut exchanges_stmt = conn.prepare(
    //     "SELECT id FROM participant_exchange WHERE participant_id = ?1",
    // )?;

    // let exchanges: Vec<i64> = exchanges_stmt.query_map(
    //     [&partipant_id],
    //     |row| row.get(0)
    // )?.filter_map(|r| r.ok()).collect();
    // let mut exclusions_stmt = conn.prepare(
    //     "SELECT excluded_participant_id FROM participant_exclusion WHERE participant_id = ?1",
    // )?;
    // let excluded_participants: Vec<i64> = exclusions_stmt.query_map(
    //     [&partipant_id],
    //     |row| row.get(0)
    // )?.filter_map(|r| r.ok()).collect();
    Ok(partipant_id)
}

pub fn reset_pairs_for_exchange(conn: &mut Connection, exchange_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM exchange_pairing WHERE exchange_id = ?1",
        [exchange_id],
    )?;
    Ok(())
}

pub fn add_exchange_pair(
    conn: &mut Connection,
    giver_id: i64,
    receiver_id: i64,
    exchange_id: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO exchange_pairing (giver_id, receiver_id, exchange_id) VALUES (?1, ?2, ?3)",
        [giver_id, receiver_id, exchange_id],
    )?;
    Ok(())
}
