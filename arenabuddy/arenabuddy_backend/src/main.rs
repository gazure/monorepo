use std::{pin::Pin, sync::Arc, time::Duration};

use anyhow::Result;
use arenabuddy_core::{
    cards::CardsDatabase,
    models::{Deck, MTGAMatch, MatchResult, Mulligan},
    replay::MatchReplay,
};
use arenabuddy_data::{ArenabuddyRepository, MatchDB};
use tokio::{
    sync::{broadcast, Mutex, RwLock},
    time::sleep,
};
use tokio_stream::{Stream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod areabuddypb {
    tonic::include_proto!("arenabuddy");
}

pub mod client;

use areabuddypb::{
    arena_buddy_service_server::{ArenaBuddyService, ArenaBuddyServiceServer},
    match_sync_request::Request as SyncRequest,
    match_sync_response::Response as SyncResponse,
    *,
};

type MatchUpdateSender = broadcast::Sender<MatchUpdate>;

#[derive(Clone)]
struct ArenaBuddyServer {
    db: Arc<RwLock<MatchDB>>,
    update_sender: MatchUpdateSender,
    active_sessions: Arc<Mutex<std::collections::HashMap<String, SessionInfo>>>,
}

struct SessionInfo {
    _client_id: String,
    _player_name: String,
    last_heartbeat: std::time::Instant,
}

impl ArenaBuddyServer {
    async fn new(database_url: Option<&str>) -> Result<Self> {
        info!("Initializing ArenaBuddy server");

        // Initialize cards database
        let cards = Arc::new(CardsDatabase::default());

        // Initialize PostgreSQL connection
        let mut db = MatchDB::new(database_url, cards).await?;
        db.initialize().await?;

        // Create broadcast channel for match updates
        let (update_sender, _) = broadcast::channel(1024);

        Ok(Self {
            db: Arc::new(RwLock::new(db)),
            update_sender,
            active_sessions: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }

    async fn process_match_data(&self, match_data: &MatchReplayData) -> Result<(), Status> {
        info!("Processing match data for match_id: {}", match_data.match_id);

        // Convert protobuf MatchReplayData to internal MatchReplay format
        // This is simplified - you'd need to properly convert the data
        let match_replay = self.convert_to_match_replay(match_data)?;

        // Write to database
        let mut db = self.db.write().await;
        db.write_replay(&match_replay)
            .await
            .map_err(|e| Status::internal(format!("Failed to save match: {}", e)))?;

        // Broadcast update to subscribers
        let update = MatchUpdate {
            r#type: match_update::UpdateType::NewMatch as i32,
            match_data: Some(match_data.clone()),
            timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        };

        // Ignore send errors (no receivers)
        let _ = self.update_sender.send(update);

        Ok(())
    }

    fn convert_to_match_replay(&self, _match_data: &MatchReplayData) -> Result<MatchReplay, Status> {
        // This would need proper implementation to convert from protobuf to internal format
        // For now, return an error indicating this needs implementation
        Err(Status::unimplemented("Match replay conversion not yet implemented"))
    }

    fn convert_match_to_proto(
        &self,
        match_: &MTGAMatch,
        decks: Vec<Deck>,
        mulligans: Vec<Mulligan>,
        results: Vec<MatchResult>,
    ) -> MatchReplayData {
        MatchReplayData {
            match_id: match_.id().to_string(),
            controller_seat_id: match_.controller_seat_id(),
            controller_player_name: match_.controller_player_name().to_string(),
            opponent_player_name: match_.opponent_player_name().to_string(),
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::from(
                match_.created_at(),
            ))),
            decks: decks.into_iter().map(|d| self.convert_deck_to_proto(d)).collect(),
            mulligans: mulligans
                .into_iter()
                .map(|m| self.convert_mulligan_to_proto(m))
                .collect(),
            match_results: results.into_iter().map(|r| self.convert_result_to_proto(r)).collect(),
            opponent_deck: None, // Would need to fetch separately
            raw_replay_data: vec![],
        }
    }

    fn convert_deck_to_proto(&self, deck: Deck) -> DeckData {
        DeckData {
            game_number: deck.game_number(),
            deck_name: deck.name().to_string(),
            mainboard: deck
                .mainboard()
                .iter()
                .map(|id| CardEntry { grp_id: *id as i32 })
                .collect(),
            sideboard: deck
                .sideboard()
                .iter()
                .map(|id| CardEntry { grp_id: *id as i32 })
                .collect(),
        }
    }

    fn convert_mulligan_to_proto(&self, mulligan: Mulligan) -> MulliganData {
        MulliganData {
            game_number: mulligan.game_number(),
            number_to_keep: mulligan.number_to_keep(),
            hand: mulligan.hand().to_string(),
            play_draw: mulligan.play_draw().to_string(),
            opponent_identity: mulligan.opponent_identity().to_string(),
            decision: mulligan.decision().to_string(),
        }
    }

    fn convert_result_to_proto(&self, result: MatchResult) -> MatchResultData {
        MatchResultData {
            game_number: result.game_number(),
            winning_team_id: result.winning_team_id(),
            result_scope: result.result_scope().to_string(),
        }
    }
}

#[tonic::async_trait]
impl ArenaBuddyService for ArenaBuddyServer {
    type SubscribeToMatchesStream = Pin<Box<dyn Stream<Item = Result<MatchUpdate, Status>> + Send>>;
    type SyncMatchesStream = Pin<Box<dyn Stream<Item = Result<MatchSyncResponse, Status>> + Send>>;

    async fn sync_matches(
        &self,
        request: Request<Streaming<MatchSyncRequest>>,
    ) -> Result<Response<Self::SyncMatchesStream>, Status> {
        info!("New sync_matches stream connection");

        let mut stream = request.into_inner();
        let server = self.clone();
        let session_id = Uuid::new_v4().to_string();

        let output = async_stream::try_stream! {
            // Wait for client hello
            if let Some(Ok(msg)) = stream.next().await {
                if let Some(SyncRequest::Hello(hello)) = msg.request {
                    info!("Client connected: {} ({})", hello.client_id, hello.player_name);

                    // Store session info
                    server.active_sessions.lock().await.insert(
                        session_id.clone(),
                        SessionInfo {
                            _client_id: hello.client_id.clone(),
                            _player_name: hello.player_name.clone(),
                            last_heartbeat: std::time::Instant::now(),
                        }
                    );

                    // Send server hello
                    yield MatchSyncResponse {
                        response: Some(SyncResponse::Hello(ServerHello {
                            session_id: session_id.clone(),
                            server_time: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                            pending_match_ids: vec![], // Could check for unsynced matches
                        })),
                    };

                    // Process incoming messages
                    while let Some(Ok(msg)) = stream.next().await {
                        match msg.request {
                            Some(SyncRequest::MatchData(match_data)) => {
                                info!("Received match data: {}", match_data.match_id);

                                // Process the match
                                match server.process_match_data(&match_data).await {
                                    Ok(_) => {
                                        yield MatchSyncResponse {
                                            response: Some(SyncResponse::Received(MatchReceived {
                                                match_id: match_data.match_id.clone(),
                                                received_at: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                                                status: match_received::ProcessingStatus::Completed as i32,
                                            })),
                                        };
                                    }
                                    Err(e) => {
                                        error!("Failed to process match: {}", e);
                                        yield MatchSyncResponse {
                                            response: Some(SyncResponse::Error(SyncError {
                                                match_id: match_data.match_id.clone(),
                                                error_message: e.to_string(),
                                                error_type: sync_error::ErrorType::DatabaseError as i32,
                                            })),
                                        };
                                    }
                                }
                            }
                            Some(SyncRequest::Heartbeat(hb)) => {
                                // Update last heartbeat time
                                if let Some(session) = server.active_sessions.lock().await.get_mut(&session_id) {
                                    session.last_heartbeat = std::time::Instant::now();
                                }

                                // Echo heartbeat
                                yield MatchSyncResponse {
                                    response: Some(SyncResponse::Heartbeat(hb)),
                                };
                            }
                            Some(SyncRequest::Ack(ack)) => {
                                info!("Received ack for match {}: {}", ack.match_id, ack.success);
                            }
                            _ => {
                                warn!("Unexpected message type");
                            }
                        }
                    }
                } else {
                    error!("Expected ClientHello as first message");
                }
            }

            // Clean up session
            server.active_sessions.lock().await.remove(&session_id);
            info!("Client disconnected: {}", session_id);
        };

        Ok(Response::new(Box::pin(output) as Self::SyncMatchesStream))
    }

    async fn upload_match(
        &self,
        request: Request<UploadMatchRequest>,
    ) -> Result<Response<UploadMatchResponse>, Status> {
        let match_data = request
            .into_inner()
            .match_data
            .ok_or_else(|| Status::invalid_argument("Missing match data"))?;

        info!("Uploading match: {}", match_data.match_id);

        match self.process_match_data(&match_data).await {
            Ok(_) => Ok(Response::new(UploadMatchResponse {
                success: true,
                match_id: match_data.match_id,
                message: "Match uploaded successfully".to_string(),
            })),
            Err(e) => Ok(Response::new(UploadMatchResponse {
                success: false,
                match_id: match_data.match_id,
                message: e.to_string(),
            })),
        }
    }

    async fn list_matches(
        &self,
        request: Request<ListMatchesRequest>,
    ) -> Result<Response<ListMatchesResponse>, Status> {
        let req = request.into_inner();
        info!("Listing matches for player: {}", req.player_name);

        let db = self.db.read().await;
        let matches = db
            .get_matches()
            .await
            .map_err(|e| Status::internal(format!("Failed to fetch matches: {}", e)))?;

        // Filter by player name if provided
        let filtered: Vec<_> = if !req.player_name.is_empty() {
            matches
                .into_iter()
                .filter(|m| {
                    m.controller_player_name() == req.player_name || m.opponent_player_name() == req.player_name
                })
                .collect()
        } else {
            matches
        };

        // Apply pagination
        let total_count = filtered.len() as i32;
        let offset = req.offset.max(0) as usize;
        let limit = req.limit.max(1).min(100) as usize;

        let paginated: Vec<_> = filtered.into_iter().skip(offset).take(limit).collect();

        // Convert to protobuf format
        let mut summaries = Vec::new();
        for match_ in paginated {
            // Get match result to determine winner
            let (_, result) = db
                .retrieve_match(match_.id())
                .await
                .map_err(|e| Status::internal(format!("Failed to fetch match result: {}", e)))?;

            let did_controller_win = result
                .map(|r| r.winning_team_id() == 1) // Assuming controller is team 1
                .unwrap_or(false);

            summaries.push(MatchSummary {
                match_id: match_.id().to_string(),
                controller_player_name: match_.controller_player_name().to_string(),
                opponent_player_name: match_.opponent_player_name().to_string(),
                created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::from(
                    match_.created_at(),
                ))),
                did_controller_win,
                games_played: 0, // Would need to count game results
            });
        }

        Ok(Response::new(ListMatchesResponse {
            matches: summaries,
            total_count,
        }))
    }

    async fn get_match(&self, request: Request<GetMatchRequest>) -> Result<Response<GetMatchResponse>, Status> {
        let match_id = request.into_inner().match_id;
        info!("Getting match details for: {}", match_id);

        let mut db = self.db.write().await;

        // Get match info
        let (match_, _) = db
            .get_match(&match_id)
            .await
            .map_err(|e| Status::not_found(format!("Match not found: {}", e)))?;

        // Get related data
        let decks = db
            .list_decklists(&match_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to fetch decks: {}", e)))?;

        let mulligans = db
            .list_mulligans(&match_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to fetch mulligans: {}", e)))?;

        let results = db
            .list_match_results(&match_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to fetch results: {}", e)))?;

        let match_data = self.convert_match_to_proto(&match_, decks, mulligans, results);

        Ok(Response::new(GetMatchResponse {
            match_data: Some(match_data),
        }))
    }

    async fn subscribe_to_matches(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeToMatchesStream>, Status> {
        let req = request.into_inner();
        info!("New subscription for player: {}", req.player_name);

        let mut receiver = self.update_sender.subscribe();
        let player_name = req.player_name;
        let match_ids: std::collections::HashSet<String> = req.match_ids.into_iter().collect();

        let output = async_stream::try_stream! {
            while let Ok(update) = receiver.recv().await {
                // Filter updates based on subscription criteria
                let should_send = if !player_name.is_empty() {
                    if let Some(ref match_data) = update.match_data {
                        match_data.controller_player_name == player_name ||
                        match_data.opponent_player_name == player_name
                    } else {
                        false
                    }
                } else if !match_ids.is_empty() {
                    if let Some(ref match_data) = update.match_data {
                        match_ids.contains(&match_data.match_id)
                    } else {
                        false
                    }
                } else {
                    true // Send all updates if no filter specified
                };

                if should_send {
                    yield update;
                }
            }
        };

        Ok(Response::new(Box::pin(output) as Self::SubscribeToMatchesStream))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("arenabuddy_backend=info".parse()?),
        )
        .init();

    // Load configuration
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").ok();
    let server_addr = std::env::var("GRPC_SERVER_ADDR")
        .unwrap_or_else(|_| "[::1]:50051".to_string())
        .parse()?;

    info!("Starting ArenaBuddy gRPC server on {}", server_addr);

    // Initialize server
    let server = ArenaBuddyServer::new(database_url.as_deref()).await?;

    // Start background task for cleaning up stale sessions
    let sessions = server.active_sessions.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            let mut sessions = sessions.lock().await;
            let now = std::time::Instant::now();
            sessions.retain(|id, info| {
                let elapsed = now.duration_since(info.last_heartbeat);
                if elapsed > Duration::from_secs(300) {
                    warn!("Removing stale session: {}", id);
                    false
                } else {
                    true
                }
            });
        }
    });

    // Start gRPC server
    Server::builder()
        .add_service(ArenaBuddyServiceServer::new(server))
        .serve(server_addr)
        .await?;

    Ok(())
}
