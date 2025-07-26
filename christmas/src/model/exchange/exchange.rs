use serde::{Deserialize, Serialize};
use chrono::Datelike;
use super::ParticipantGraph;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExchangeAppConfig {
    pub exchanges: Vec<Exchange>,
    pub participants: Vec<Participant>,
}

impl ExchangeAppConfig {
    pub fn pools(self) -> Vec<ExchangePool> {
        self.exchanges
            .into_iter()
            .map(|exchange| {
                let participants = self
                    .participants
                    .iter()
                    .filter(|p| p.exchange_pools.contains(&exchange.name))
                    .cloned()
                    .collect();
                ExchangePool {
                    exchange,
                    participants,
                }
            })
            .collect()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub name: String,
    pub exchange_pools: Vec<String>,
    pub exclusions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exchange {
    pub name: String,
    pub letters: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangePool {
    pub exchange: Exchange,
    pub participants: Vec<Participant>,
}

impl ExchangePool {
    pub fn generate_pairings(&self) -> ExchangeResult {
        let graph = ParticipantGraph::from_participants(self.participants.clone());
        let pairs = graph.build_exchange();
        
        let pairings = pairs
            .into_iter()
            .map(|(giver, receiver)| ExchangePairing { giver, receiver })
            .collect();

        // Calculate year and year letter
        let year = chrono::Utc::now().year();
        let year_letter = if let Some(letters) = &self.exchange.letters {
            let year_index = (year - 2024) as usize % letters.len();
            letters.chars().nth(year_index).unwrap_or('A')
        } else {
            'A'
        };

        ExchangeResult {
            pairings,
            year_letter,
            year,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExchangePairing {
    pub giver: String,
    pub receiver: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExchangeResult {
    pub pairings: Vec<ExchangePairing>,
    pub year_letter: char,
    pub year: i32,
}
