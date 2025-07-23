use crate::giftexchange::ExchangePool;

#[derive(Debug, Default, Clone)]
pub struct Participant {
    pub name: String,
    pub exchange_pools: Vec<ExchangePool>,
    pub exclusions: Vec<String>,
}

impl Participant {
    pub fn new(
        name: String,
        exchange_pools: Vec<ExchangePool>,
        exclusions: Vec<&str>,
    ) -> Participant {
        let exclusions = exclusions.iter().map(|s| s.to_string()).collect();
        Participant {
            name,
            exchange_pools,
            exclusions,
        }
    }
}
