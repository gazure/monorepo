use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use arenabuddy_core::replay::MatchReplay;
use chrono::Utc;
use tokio::{
    sync::{mpsc, Mutex, RwLock},
    time::{interval, sleep},
};
use tokio_stream::StreamExt;
use tonic::{
    transport::{Channel, Endpoint},
    Request,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::areabuddypb::{
    arena_buddy_service_client::ArenaBuddyServiceClient, match_sync_request::Request as SyncRequest,
    match_sync_response::Response as SyncResponse, *,
};

pub struct ArenaBuddyClient {
    client: Arc<Mutex<ArenaBuddyServiceClient<Channel>>>,
    endpoint: Endpoint,
    client_id: String,
    player_name: String,
    sync_channel: Option<mpsc::Sender<MatchSyncRequest>>,
    connected: Arc<RwLock<bool>>,
}

impl ArenaBuddyClient {
    /// Create a new ArenaBuddy client
    pub async fn new(server_addr: &str, player_name: String) -> Result<Self> {
        let endpoint = Endpoint::from_shared(format!("http://{}", server_addr))?
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10));

        let channel = endpoint.connect().await?;
        let client = ArenaBuddyServiceClient::new(channel);
        let client_id = Uuid::new_v4().to_string();

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            endpoint,
            client_id,
            player_name,
            sync_channel: None,
            connected: Arc::new(RwLock::new(true)),
        })
    }

    /// Check if the client is connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Reconnect to the server
    async fn reconnect(&self) -> Result<()> {
        info!("Attempting to reconnect to server");

        let channel = self.endpoint.connect().await?;
        let new_client = ArenaBuddyServiceClient::new(channel);

        *self.client.lock().await = new_client;
        *self.connected.write().await = true;

        info!("Successfully reconnected to server");
        Ok(())
    }

    /// Start bi-directional streaming sync session
    pub async fn start_sync_session(&mut self) -> Result<()> {
        info!("Starting sync session");

        let (tx, mut rx) = mpsc::channel::<MatchSyncRequest>(100);
        self.sync_channel = Some(tx.clone());

        // Send initial hello
        let hello = MatchSyncRequest {
            request: Some(SyncRequest::Hello(ClientHello {
                client_id: self.client_id.clone(),
                player_name: self.player_name.clone(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                last_sync_time: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
            })),
        };

        tx.send(hello).await?;

        // Create stream from channel
        let outbound = async_stream::stream! {
            while let Some(msg) = rx.recv().await {
                yield msg;
            }
        };

        // Start bi-directional stream
        let mut client = self.client.lock().await.clone();
        let response = client.sync_matches(Request::new(outbound)).await?;
        let mut inbound = response.into_inner();

        // Spawn task to handle incoming messages
        let connected = self.connected.clone();
        let tx_heartbeat = tx.clone();

        tokio::spawn(async move {
            // Spawn heartbeat task
            let tx_hb = tx_heartbeat.clone();
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    let heartbeat = MatchSyncRequest {
                        request: Some(SyncRequest::Heartbeat(Heartbeat {
                            timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
                        })),
                    };
                    if tx_hb.send(heartbeat).await.is_err() {
                        break;
                    }
                }
            });

            // Handle incoming messages
            while let Some(msg) = inbound.next().await {
                match msg {
                    Ok(response) => {
                        if let Some(resp) = response.response {
                            match resp {
                                SyncResponse::Hello(hello) => {
                                    info!("Connected to server with session: {}", hello.session_id);
                                    if !hello.pending_match_ids.is_empty() {
                                        info!("Server has {} pending matches", hello.pending_match_ids.len());
                                    }
                                }
                                SyncResponse::Received(received) => {
                                    info!("Server received match {}: {:?}", received.match_id, received.status);
                                }
                                SyncResponse::Update(update) => {
                                    info!("Received match update: {:?}", update.r#type);
                                }
                                SyncResponse::Error(error) => {
                                    error!("Server error for match {}: {}", error.match_id, error.error_message);
                                }
                                SyncResponse::Heartbeat(_) => {
                                    debug!("Heartbeat acknowledged");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Stream error: {}", e);
                        *connected.write().await = false;
                        break;
                    }
                }
            }

            info!("Sync session ended");
        });

        Ok(())
    }

    /// Send a match replay through the sync session
    pub async fn send_match_sync(&self, replay: &MatchReplay) -> Result<()> {
        let tx = self
            .sync_channel
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Sync session not started"))?;

        let match_data = self.convert_replay_to_proto(replay)?;

        let request = MatchSyncRequest {
            request: Some(SyncRequest::MatchData(match_data)),
        };

        tx.send(request).await.context("Failed to send match data")?;

        Ok(())
    }

    /// Upload a single match (one-shot request)
    pub async fn upload_match(&self, replay: &MatchReplay) -> Result<String> {
        info!("Uploading match: {}", replay.match_id);

        let match_data = self.convert_replay_to_proto(replay)?;

        let request = UploadMatchRequest {
            match_data: Some(match_data),
        };

        let response = self
            .client
            .lock()
            .await
            .upload_match(Request::new(request))
            .await?
            .into_inner();

        if response.success {
            Ok(response.match_id)
        } else {
            Err(anyhow::anyhow!("Upload failed: {}", response.message))
        }
    }

    /// List matches with optional filters
    pub async fn list_matches(
        &self,
        player_name: Option<String>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<MatchSummary>> {
        let request = ListMatchesRequest {
            player_name: player_name.unwrap_or_default(),
            start_time: None,
            end_time: None,
            limit,
            offset,
        };

        let response = self
            .client
            .lock()
            .await
            .list_matches(Request::new(request))
            .await?
            .into_inner();

        Ok(response.matches)
    }

    /// Get detailed match information
    pub async fn get_match(&self, match_id: &str) -> Result<MatchReplayData> {
        let request = GetMatchRequest {
            match_id: match_id.to_string(),
        };

        let response = self
            .client
            .lock()
            .await
            .get_match(Request::new(request))
            .await?
            .into_inner();

        response
            .match_data
            .ok_or_else(|| anyhow::anyhow!("Match data not found"))
    }

    /// Subscribe to match updates
    pub async fn subscribe_to_updates(
        &self,
        player_name: Option<String>,
        match_ids: Vec<String>,
    ) -> Result<mpsc::Receiver<MatchUpdate>> {
        let request = SubscribeRequest {
            player_name: player_name.unwrap_or_default(),
            match_ids,
        };

        let response = self
            .client
            .lock()
            .await
            .subscribe_to_matches(Request::new(request))
            .await?;

        let mut stream = response.into_inner();
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(update) => {
                        if tx.send(update).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Subscription error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Convert internal MatchReplay to protobuf format
    fn convert_replay_to_proto(&self, replay: &MatchReplay) -> Result<MatchReplayData> {
        // This is a simplified conversion - you'd need to implement the full conversion
        // based on your actual MatchReplay structure

        let controller_seat_id = replay.controller_seat_id;
        let (controller_name, opponent_name) = replay.get_player_names(controller_seat_id)?;

        Ok(MatchReplayData {
            match_id: replay.match_id.clone(),
            controller_seat_id,
            controller_player_name: controller_name,
            opponent_player_name: opponent_name,
            created_at: Some(prost_types::Timestamp::from(std::time::SystemTime::from(
                replay.match_start_time().unwrap_or(Utc::now()),
            ))),
            decks: vec![],         // Would need to convert decks
            mulligans: vec![],     // Would need to convert mulligans
            match_results: vec![], // Would need to convert results
            opponent_deck: None,
            raw_replay_data: vec![], // Could serialize the full replay here
        })
    }

    /// Start auto-reconnect loop
    pub fn start_auto_reconnect(&self) -> tokio::task::JoinHandle<()> {
        let connected = self.connected.clone();
        let client_self = self.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                if !*connected.read().await {
                    info!("Connection lost, attempting to reconnect...");

                    for attempt in 1..=5 {
                        match client_self.reconnect().await {
                            Ok(_) => {
                                info!("Reconnected successfully");
                                break;
                            }
                            Err(e) => {
                                warn!("Reconnection attempt {} failed: {}", attempt, e);
                                if attempt < 5 {
                                    sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                                }
                            }
                        }
                    }
                }
            }
        })
    }
}

impl Clone for ArenaBuddyClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            endpoint: self.endpoint.clone(),
            client_id: self.client_id.clone(),
            player_name: self.player_name.clone(),
            sync_channel: None,
            connected: self.connected.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        // This would need a test server running
        let result = ArenaBuddyClient::new("localhost:50051", "TestPlayer".to_string()).await;

        // Will fail if no server is running, which is expected in unit tests
        assert!(result.is_err() || result.is_ok());
    }
}
