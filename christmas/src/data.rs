use crate::{exchange::Participant, giftexchange::ExchangePool};

/// Returns all participants for the gift exchange
pub fn get_all_participants() -> Vec<Participant> {
    vec![
        Participant::new(
            "Claire".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Duncan", "Chris"],
        ),
        Participant::new(
            "Grant".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Noel"],
        ),
        Participant::new(
            "Anne".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Eric", "Kari"],
        ),
        Participant::new(
            "Duncan".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Claire", "Chris"],
        ),
        Participant::new(
            "Noel".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["K-Lee", "Claire"],
        ),
        Participant::new(
            "K-Lee".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Noel", "Jim"],
        ),
        Participant::new(
            "Steve".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Linda", "Duncan"],
        ),
        Participant::new(
            "Linda".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Steve", "Alec"],
        ),
        Participant::new(
            "Chris".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Eric"],
        ),
        Participant::new(
            "Jim".to_string(),
            vec![ExchangePool::Grabergishimazureson],
            vec!["Kari", "Anne"],
        ),
        Participant::new(
            "Kari".to_string(),
            vec![ExchangePool::Grabergishimazureson],
            vec!["Jim", "Linda"],
        ),
        Participant::new(
            "Meaghann".to_string(),
            vec![ExchangePool::Grabergishimazureson],
            vec!["Steve"],
        ),
        Participant::new(
            "Alec".to_string(),
            vec![ExchangePool::Grabergishimazureson],
            vec!["Meaghann"],
        ),
        Participant::new(
            "Eric".to_string(),
            vec![ExchangePool::IslandLife, ExchangePool::Grabergishimazureson],
            vec!["Anne", "K-Lee"],
        ),
        Participant::new(
            "Stella".to_string(),
            vec![ExchangePool::Pets],
            vec!["Daisy"],
        ),
        Participant::new("Bailey".to_string(), vec![ExchangePool::Pets], vec!["Luca"]),
        Participant::new(
            "Kitty".to_string(),
            vec![ExchangePool::Pets],
            vec!["Bailey"],
        ),
        Participant::new(
            "Charlie".to_string(),
            vec![ExchangePool::Pets],
            vec!["Kona"],
        ),
        Participant::new("Astra".to_string(), vec![ExchangePool::Pets], vec!["Lily"]),
        Participant::new(
            "Freya".to_string(),
            vec![ExchangePool::Pets],
            vec!["Stella"],
        ),
        Participant::new("Lily".to_string(), vec![ExchangePool::Pets], vec!["Kitty"]),
        Participant::new("Daisy".to_string(), vec![ExchangePool::Pets], vec!["Astra"]),
        Participant::new(
            "Luca".to_string(),
            vec![ExchangePool::Pets],
            vec!["Charlie"],
        ),
        Participant::new("Kona".to_string(), vec![ExchangePool::Pets], vec!["Freya"]),
    ]
}

/// Returns participants filtered by exchange pool
pub fn get_participants_by_pool(pool: ExchangePool) -> Vec<Participant> {
    get_all_participants()
        .into_iter()
        .filter(|p| p.exchange_pools.contains(&pool))
        .collect()
}
