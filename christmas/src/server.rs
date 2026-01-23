use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Participant {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Exclusion {
    pub id: i32,
    pub participant_a: Participant,
    pub participant_b: Participant,
    pub reason: Option<String>,
}

/// Snapshot of an exclusion for archiving in exchanges
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExclusionSnapshot {
    pub a: String,
    pub b: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Exchange {
    pub id: i32,
    pub year: i32,
    pub letter: Option<char>,
    pub participants: Vec<String>,
    pub exclusions: Vec<ExclusionSnapshot>,
    pub pairings: Vec<Pairing>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pairing {
    pub giver: String,
    pub receiver: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExchangeResult {
    pub exchange_id: i32,
    pub year: i32,
    pub letter: Option<char>,
    pub pairings: Vec<Pairing>,
}

#[cfg(feature = "server")]
fn db_err(e: sqlx::Error) -> ServerFnError {
    ServerFnError::new(e.to_string())
}

// ============================================================================
// Participant Functions
// ============================================================================

#[server]
pub async fn list_participants() -> Result<Vec<Participant>, ServerFnError> {
    use sqlx::FromRow;

    #[derive(FromRow)]
    struct Row {
        id: i32,
        name: String,
    }

    let pool = crate::pool()?;
    let rows: Vec<Row> = sqlx::query_as("SELECT id, name FROM participant ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(db_err)?;

    Ok(rows
        .into_iter()
        .map(|r| Participant { id: r.id, name: r.name })
        .collect())
}

#[server]
pub async fn add_participant(name: String) -> Result<Participant, ServerFnError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Name cannot be empty"));
    }

    let pool = crate::pool()?;
    let row: (i32,) = sqlx::query_as("INSERT INTO participant (name) VALUES ($1) RETURNING id")
        .bind(&name)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;

    Ok(Participant { id: row.0, name })
}

#[server]
pub async fn remove_participant(id: i32) -> Result<(), ServerFnError> {
    let pool = crate::pool()?;
    sqlx::query("DELETE FROM participant WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    Ok(())
}

// ============================================================================
// Exclusion Functions
// ============================================================================

#[server]
pub async fn list_exclusions() -> Result<Vec<Exclusion>, ServerFnError> {
    use sqlx::FromRow;

    #[derive(FromRow)]
    struct Row {
        id: i32,
        a_id: i32,
        a_name: String,
        b_id: i32,
        b_name: String,
        reason: Option<String>,
    }

    let pool = crate::pool()?;
    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT 
            e.id,
            pa.id as a_id, pa.name as a_name,
            pb.id as b_id, pb.name as b_name,
            e.reason
        FROM exclusion e
        JOIN participant pa ON e.participant_a_id = pa.id
        JOIN participant pb ON e.participant_b_id = pb.id
        ORDER BY pa.name, pb.name
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    Ok(rows
        .into_iter()
        .map(|r| Exclusion {
            id: r.id,
            participant_a: Participant {
                id: r.a_id,
                name: r.a_name,
            },
            participant_b: Participant {
                id: r.b_id,
                name: r.b_name,
            },
            reason: r.reason,
        })
        .collect())
}

#[server]
pub async fn add_exclusion(
    participant_a_id: i32,
    participant_b_id: i32,
    reason: Option<String>,
) -> Result<Exclusion, ServerFnError> {
    if participant_a_id == participant_b_id {
        return Err(ServerFnError::new("Cannot create exclusion with same participant"));
    }

    // Normalize order: smaller ID first
    let (a_id, b_id) = if participant_a_id < participant_b_id {
        (participant_a_id, participant_b_id)
    } else {
        (participant_b_id, participant_a_id)
    };

    let pool = crate::pool()?;

    // Get participant names
    let a: (String,) = sqlx::query_as("SELECT name FROM participant WHERE id = $1")
        .bind(a_id)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
    let b: (String,) = sqlx::query_as("SELECT name FROM participant WHERE id = $1")
        .bind(b_id)
        .fetch_one(pool)
        .await
        .map_err(db_err)?;

    let row: (i32,) = sqlx::query_as(
        "INSERT INTO exclusion (participant_a_id, participant_b_id, reason) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(a_id)
    .bind(b_id)
    .bind(&reason)
    .fetch_one(pool)
    .await
    .map_err(db_err)?;

    Ok(Exclusion {
        id: row.0,
        participant_a: Participant { id: a_id, name: a.0 },
        participant_b: Participant { id: b_id, name: b.0 },
        reason,
    })
}

#[server]
pub async fn remove_exclusion(id: i32) -> Result<(), ServerFnError> {
    let pool = crate::pool()?;
    sqlx::query("DELETE FROM exclusion WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(db_err)?;
    Ok(())
}

// ============================================================================
// Letter Functions
// ============================================================================

#[server]
pub async fn list_excluded_letters() -> Result<Vec<char>, ServerFnError> {
    let pool = crate::pool()?;
    let rows: Vec<(String,)> = sqlx::query_as("SELECT letter FROM excluded_letter ORDER BY letter")
        .fetch_all(pool)
        .await
        .map_err(db_err)?;

    Ok(rows.into_iter().filter_map(|r| r.0.chars().next()).collect())
}

#[server]
pub async fn set_excluded_letters(letters: Vec<char>) -> Result<(), ServerFnError> {
    let pool = crate::pool()?;

    // Clear existing and insert new
    sqlx::query("DELETE FROM excluded_letter")
        .execute(pool)
        .await
        .map_err(db_err)?;

    for letter in letters {
        if letter.is_ascii_uppercase() {
            sqlx::query("INSERT INTO excluded_letter (letter) VALUES ($1)")
                .bind(letter.to_string())
                .execute(pool)
                .await
                .map_err(db_err)?;
        }
    }

    Ok(())
}

// ============================================================================
// Exchange Functions
// ============================================================================

#[server]
pub async fn run_exchange(year: i32, include_letter: bool) -> Result<ExchangeResult, ServerFnError> {
    use sqlx::types::Json;

    use crate::matching;

    let pool = crate::pool()?;

    // Get all participants
    let participants: Vec<(String,)> = sqlx::query_as("SELECT name FROM participant ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(db_err)?;

    if participants.len() < 2 {
        return Err(ServerFnError::new("Need at least 2 participants for an exchange"));
    }

    let participant_names: Vec<String> = participants.into_iter().map(|r| r.0).collect();

    // Get all exclusions for matching algorithm
    let exclusion_rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT pa.name, pb.name, e.reason
        FROM exclusion e
        JOIN participant pa ON e.participant_a_id = pa.id
        JOIN participant pb ON e.participant_b_id = pb.id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_err)?;

    // Create exclusion pairs for the matching algorithm
    let exclusion_pairs: Vec<(String, String)> =
        exclusion_rows.iter().map(|(a, b, _)| (a.clone(), b.clone())).collect();

    // Build the exchange
    let pairings = matching::build_exchange(&participant_names, &exclusion_pairs)
        .ok_or_else(|| ServerFnError::new("Could not generate valid pairings with given exclusions"))?;

    // Select letter if requested
    let letter = if include_letter {
        let excluded: Vec<(String,)> = sqlx::query_as("SELECT letter FROM excluded_letter")
            .fetch_all(pool)
            .await
            .map_err(db_err)?;
        let excluded_chars: Vec<char> = excluded.into_iter().filter_map(|r| r.0.chars().next()).collect();
        matching::select_letter(&excluded_chars)
    } else {
        None
    };

    // Create snapshots for archiving
    let exclusion_snapshots: Vec<ExclusionSnapshot> = exclusion_rows
        .into_iter()
        .map(|(a, b, reason)| ExclusionSnapshot { a, b, reason })
        .collect();

    let pairing_snapshots: Vec<Pairing> = pairings
        .into_iter()
        .map(|(giver, receiver)| Pairing { giver, receiver })
        .collect();

    // Check if exchange for this year already exists
    let existing: Option<(i32,)> = sqlx::query_as("SELECT id FROM exchange WHERE year = $1")
        .bind(year)
        .fetch_optional(pool)
        .await
        .map_err(db_err)?;

    let exchange_id = if let Some((id,)) = existing {
        // Update existing exchange with new snapshots
        sqlx::query(
            r#"
            UPDATE exchange 
            SET letter = $1, participants = $2, exclusions = $3, pairings = $4
            WHERE id = $5
            "#,
        )
        .bind(letter.map(|c| c.to_string()))
        .bind(Json(&participant_names))
        .bind(Json(&exclusion_snapshots))
        .bind(Json(&pairing_snapshots))
        .bind(id)
        .execute(pool)
        .await
        .map_err(db_err)?;
        id
    } else {
        // Create new exchange with snapshots
        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO exchange (year, letter, participants, exclusions, pairings) 
            VALUES ($1, $2, $3, $4, $5) 
            RETURNING id
            "#,
        )
        .bind(year)
        .bind(letter.map(|c| c.to_string()))
        .bind(Json(&participant_names))
        .bind(Json(&exclusion_snapshots))
        .bind(Json(&pairing_snapshots))
        .fetch_one(pool)
        .await
        .map_err(db_err)?;
        row.0
    };

    Ok(ExchangeResult {
        exchange_id,
        year,
        letter,
        pairings: pairing_snapshots,
    })
}

#[server]
pub async fn list_exchanges() -> Result<Vec<Exchange>, ServerFnError> {
    use sqlx::{FromRow, types::Json};

    #[derive(FromRow)]
    struct ExchangeRow {
        id: i32,
        year: i32,
        letter: Option<String>,
        participants: Json<Vec<String>>,
        exclusions: Json<Vec<ExclusionSnapshot>>,
        pairings: Json<Vec<Pairing>>,
    }

    let pool = crate::pool()?;

    let rows: Vec<ExchangeRow> =
        sqlx::query_as("SELECT id, year, letter, participants, exclusions, pairings FROM exchange ORDER BY year DESC")
            .fetch_all(pool)
            .await
            .map_err(db_err)?;

    Ok(rows
        .into_iter()
        .map(|r| Exchange {
            id: r.id,
            year: r.year,
            letter: r.letter.and_then(|s| s.chars().next()),
            participants: r.participants.0,
            exclusions: r.exclusions.0,
            pairings: r.pairings.0,
        })
        .collect())
}
