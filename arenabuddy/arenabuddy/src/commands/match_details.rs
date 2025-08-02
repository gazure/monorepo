use std::sync::Arc;

use arenabuddy_core::display::{
    deck::{DeckDisplayRecord, Difference},
    game::GameResultDisplay,
    match_details::MatchDetails,
    mulligan::Mulligan,
};
use arenabuddy_data::MatchDB;
use tauri::State;
use tokio::sync::Mutex;
use tracing::{error, info};

#[tauri::command]
pub async fn command_match_details(
    match_id: String,
    db: State<'_, Arc<Mutex<MatchDB>>>,
) -> Result<MatchDetails, ()> {
    let db = db.inner().lock().await;
    info!("looking for match {match_id}");

    let (mtga_match, result) = db.get_match(&match_id).await.unwrap_or_default();

    let mut match_details = MatchDetails {
        id: match_id.clone(),
        controller_seat_id: mtga_match.controller_seat_id(),
        controller_player_name: mtga_match.controller_player_name().to_string(),
        opponent_player_name: mtga_match.opponent_player_name().to_string(),
        created_at: mtga_match.created_at(),
        did_controller_win: result.is_some_and(|r| r.is_winner(mtga_match.controller_seat_id())),
        ..Default::default()
    };

    match_details.decklists = db.get_decklists(&match_id).await.unwrap_or_default();

    match_details.primary_decklist = match_details
        .decklists
        .first()
        .map(|primary_decklist| DeckDisplayRecord::from_decklist(primary_decklist, &db.cards));

    match_details.decklists.windows(2).for_each(|pair| {
        if let [prev, next] = pair {
            let diff = Difference::diff(prev, next, &db.cards);
            match_details
                .differences
                .get_or_insert_with(Vec::new)
                .push(diff);
        }
    });

    let raw_mulligans = db.get_mulligans(&match_id).await.unwrap_or_else(|e| {
        error!("Error retrieving Mulligans: {}", e);
        Vec::default()
    });

    match_details.mulligans = raw_mulligans
        .iter()
        .map(|mulligan| Mulligan::from_model(mulligan, &db.cards))
        .collect();

    match_details.mulligans.sort();

    match_details.game_results = db
        .get_match_results(&match_id)
        .await
        .unwrap_or_else(|e| {
            error!("Error retrieving game results: {}", e);
            Vec::default()
        })
        .iter()
        .map(|mr| {
            GameResultDisplay::from_match_result(
                mr,
                match_details.controller_seat_id,
                &match_details.controller_player_name,
                &match_details.opponent_player_name,
            )
        })
        .collect();
    Ok(match_details)
}
