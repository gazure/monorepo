use std::{path::PathBuf, sync::Arc, time::Duration};

use arenabuddy_core::player_log::{
    ingest::{IngestionConfig, IngestionEvent, LogIngestionService},
    replay::MatchReplay,
};
use arenabuddy_data::{DirectoryStorage, MatchDB};
use tokio::sync::Mutex;
use tracingx::{error, info};

/// Adapter that wraps an Arc<Mutex<Option<DirectoryStorage>>> for the `ReplayWriter` trait
/// TODO: rethink this
struct DirectoryStorageAdapter {
    storage: Arc<Mutex<Option<DirectoryStorage>>>,
}

impl DirectoryStorageAdapter {
    fn new(storage: Arc<Mutex<Option<DirectoryStorage>>>) -> Self {
        Self { storage }
    }
}

#[async_trait::async_trait]
impl arenabuddy_core::player_log::ingest::ReplayWriter for DirectoryStorageAdapter {
    async fn write(&mut self, replay: &MatchReplay) -> arenabuddy_core::Result<()> {
        let mut storage = self.storage.lock().await;
        if let Some(dir) = storage.as_mut() {
            dir.write(replay)
                .await
                .map_err(|e| arenabuddy_core::Error::StorageError(e.to_string()))
        } else {
            Ok(())
        }
    }
}

/// Type alias for cleaner async callback syntax
type PinnedFuture = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

/// Helper function to handle ingestion events
fn handle_ingestion_event(event: IngestionEvent, log_collector: Arc<Mutex<Vec<String>>>) -> PinnedFuture {
    Box::pin(async move {
        if let IngestionEvent::ParseError(error) = event {
            let mut collector = log_collector.lock().await;
            collector.push(error);
        }
    })
}

pub async fn start(
    db: MatchDB,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) {
    info!("Initializing log ingestion service");

    // Configure the ingestion service
    let config = IngestionConfig::new(player_log_path.clone())
        .with_follow(true)
        .with_poll_interval(Duration::from_secs(1))
        .with_rotation_watch(true);

    // Create the service
    let service = match LogIngestionService::new(config).await {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to create log ingestion service: {}", e);
            return;
        }
    };

    // Add database writer
    let service = service.add_writer(Box::new(db.clone())).add_draft_writer(Box::new(db));

    // Add directory storage writer
    let dir_adapter = DirectoryStorageAdapter::new(debug_dir.clone());
    let service = service.add_writer(Box::new(dir_adapter));

    // Set up event callback to handle ingestion events
    let event_callback = Arc::new(move |event: IngestionEvent| handle_ingestion_event(event, log_collector.clone()));

    let service = service.with_event_callback(event_callback);

    // Start the service
    if let Err(e) = service.start().await {
        error!("Log processing failed: {}", e);
    }
}
