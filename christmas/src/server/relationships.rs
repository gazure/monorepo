use dioxus::prelude::*;

use crate::model::{Relationship, RelationshipKind};

#[server]
pub async fn list_relationships() -> Result<Vec<Relationship>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: i32,
        a_id: i32,
        a_name: String,
        b_id: i32,
        b_name: String,
        kind: String,
        note: Option<String>,
    }

    use crate::model::Participant;

    let db = crate::pool().await?;
    let rows: Vec<Row> = sqlx::query_as(
        r"SELECT e.id,
                 pa.id AS a_id, pa.name AS a_name,
                 pb.id AS b_id, pb.name AS b_name,
                 e.kind::text AS kind,
                 e.reason AS note
          FROM exclusion e
          JOIN participant pa ON e.participant_a_id = pa.id
          JOIN participant pb ON e.participant_b_id = pb.id
          ORDER BY e.kind, pa.name, pb.name",
    )
    .fetch_all(db)
    .await
    .map_err(super::db_err)?;

    Ok(rows
        .into_iter()
        .map(|r| Relationship {
            id: r.id,
            participant_a: Participant {
                id: r.a_id,
                name: r.a_name,
            },
            participant_b: Participant {
                id: r.b_id,
                name: r.b_name,
            },
            kind: RelationshipKind::from_db(&r.kind),
            note: r.note,
        })
        .collect())
}

#[server]
pub async fn add_relationship(
    first: i32,
    second: i32,
    kind: RelationshipKind,
    note: Option<String>,
) -> Result<Relationship, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct NameRow {
        id: i32,
        name: String,
    }

    use crate::model::Participant;

    if first == second {
        return Err(ServerFnError::new("A participant cannot be related to themselves"));
    }

    // The table's CHECK constraint requires the lower id first.
    let (lower, higher) = if first < second {
        (first, second)
    } else {
        (second, first)
    };

    let db = crate::pool().await?;

    let names: Vec<NameRow> = sqlx::query_as("SELECT id, name FROM participant WHERE id = ANY($1)")
        .bind(vec![lower, higher])
        .fetch_all(db)
        .await
        .map_err(super::db_err)?;

    let find = |id: i32| {
        names
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.name.clone())
            .ok_or_else(|| ServerFnError::new(format!("Participant {id} not found")))
    };
    let lower_name = find(lower)?;
    let higher_name = find(higher)?;

    let row: (i32,) = sqlx::query_as(
        r"INSERT INTO exclusion (participant_a_id, participant_b_id, kind, reason)
          VALUES ($1, $2, $3::relationship_kind, $4)
          ON CONFLICT (participant_a_id, participant_b_id)
          DO UPDATE SET kind = EXCLUDED.kind, reason = EXCLUDED.reason
          RETURNING id",
    )
    .bind(lower)
    .bind(higher)
    .bind(kind.as_db())
    .bind(&note)
    .fetch_one(db)
    .await
    .map_err(super::db_err)?;

    Ok(Relationship {
        id: row.0,
        participant_a: Participant {
            id: lower,
            name: lower_name,
        },
        participant_b: Participant {
            id: higher,
            name: higher_name,
        },
        kind,
        note,
    })
}

#[server]
pub async fn remove_relationship(id: i32) -> Result<(), ServerFnError> {
    let db = crate::pool().await?;
    sqlx::query("DELETE FROM exclusion WHERE id = $1")
        .bind(id)
        .execute(db)
        .await
        .map_err(super::db_err)?;
    Ok(())
}
