use std::fmt::Display;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ExchangePool {
    IslandLife,
    Grabergishimazureson,
    Pets,
}

impl Display for ExchangePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExchangePool::IslandLife => write!(f, "Island Life"),
            ExchangePool::Grabergishimazureson => write!(f, "Grabergishimazureson"),
            ExchangePool::Pets => write!(f, "Pets"),
        }
    }
}
