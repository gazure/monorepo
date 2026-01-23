use std::collections::{HashMap, HashSet};

/// Builds a gift exchange matching using a Hamiltonian cycle algorithm.
///
/// Takes a list of participant names and a list of exclusion pairs.
/// Returns a list of (giver, receiver) pairings where everyone gives
/// and receives exactly once, respecting the exclusion rules.
pub fn build_exchange(participants: &[String], exclusions: &[(String, String)]) -> Option<Vec<(String, String)>> {
    if participants.len() < 2 {
        return None;
    }

    let graph = ParticipantGraph::new(participants, exclusions);
    graph.build_exchange()
}

/// Selects a random letter from A-Z, excluding the specified letters.
pub fn select_letter(excluded: &[char]) -> Option<char> {
    let available: Vec<char> = ('A'..='Z').filter(|c| !excluded.contains(c)).collect();
    if available.is_empty() {
        return None;
    }
    let idx = fastrand::usize(..available.len());
    Some(available[idx])
}

#[derive(Debug)]
struct ParticipantGraph {
    edges: HashMap<String, Vec<String>>,
    participants: Vec<String>,
}

impl ParticipantGraph {
    fn new(participants: &[String], exclusions: &[(String, String)]) -> Self {
        let exclusion_set: HashSet<(&str, &str)> = exclusions
            .iter()
            .flat_map(|(a, b)| [(a.as_str(), b.as_str()), (b.as_str(), a.as_str())])
            .collect();

        let mut edges: HashMap<String, Vec<String>> = HashMap::new();

        for giver in participants {
            let mut possible_receivers: Vec<String> = participants
                .iter()
                .filter(|receiver| *receiver != giver && !exclusion_set.contains(&(giver.as_str(), receiver.as_str())))
                .cloned()
                .collect();
            fastrand::shuffle(&mut possible_receivers);
            edges.insert(giver.clone(), possible_receivers);
        }

        Self {
            edges,
            participants: participants.to_vec(),
        }
    }

    fn build_exchange(&self) -> Option<Vec<(String, String)>> {
        let num_participants = self.participants.len();

        // Try multiple times with different random starting points
        for _attempt in 0..100 {
            let mut participants_list = self.participants.clone();
            fastrand::shuffle(&mut participants_list);

            if let Some(cycle) = self.find_hamiltonian_cycle(&participants_list[0], num_participants) {
                let mut exchange = Vec::with_capacity(num_participants);
                for i in 0..cycle.len() {
                    let next = (i + 1) % cycle.len();
                    exchange.push((cycle[i].clone(), cycle[next].clone()));
                }
                return Some(exchange);
            }
        }

        self.fallback_exchange()
    }

    fn find_hamiltonian_cycle(&self, start: &str, target_length: usize) -> Option<Vec<String>> {
        let mut path = vec![start.to_string()];
        let mut visited = HashSet::new();
        visited.insert(start.to_string());

        if self.dfs_hamiltonian(&mut path, &mut visited, target_length, start) {
            Some(path)
        } else {
            None
        }
    }

    fn dfs_hamiltonian(
        &self,
        path: &mut Vec<String>,
        visited: &mut HashSet<String>,
        target_length: usize,
        start: &str,
    ) -> bool {
        if path.len() == target_length {
            let last = path.last().unwrap();
            return self
                .edges
                .get(last)
                .is_some_and(|edges| edges.contains(&start.to_string()));
        }

        let current = path.last().unwrap().clone();
        if let Some(edges) = self.edges.get(&current) {
            let mut shuffled_edges = edges.clone();
            fastrand::shuffle(&mut shuffled_edges);

            for next in shuffled_edges {
                if !visited.contains(&next) {
                    path.push(next.clone());
                    visited.insert(next.clone());

                    if self.dfs_hamiltonian(path, visited, target_length, start) {
                        return true;
                    }

                    path.pop();
                    visited.remove(&next);
                }
            }
        }

        false
    }

    fn fallback_exchange(&self) -> Option<Vec<(String, String)>> {
        let mut givers = self.participants.clone();
        let mut receivers = self.participants.clone();
        let mut exchange = vec![];

        fastrand::shuffle(&mut givers);
        fastrand::shuffle(&mut receivers);

        for giver in &givers {
            for (idx, receiver) in receivers.iter().enumerate() {
                if giver != receiver && self.can_give_to(giver, receiver) {
                    exchange.push((giver.clone(), receiver.clone()));
                    receivers.remove(idx);
                    break;
                }
            }
        }

        if exchange.len() == givers.len() {
            Some(exchange)
        } else {
            None
        }
    }

    fn can_give_to(&self, giver: &str, receiver: &str) -> bool {
        self.edges
            .get(giver)
            .is_some_and(|edges| edges.contains(&receiver.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_exchange() {
        let participants = vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()];
        let exclusions = vec![];

        let result = build_exchange(&participants, &exclusions);
        assert!(result.is_some());

        let pairings = result.unwrap();
        assert_eq!(pairings.len(), 3);

        // Everyone gives once
        let givers: HashSet<_> = pairings.iter().map(|(g, _)| g.as_str()).collect();
        assert_eq!(givers.len(), 3);

        // Everyone receives once
        let receivers: HashSet<_> = pairings.iter().map(|(_, r)| r.as_str()).collect();
        assert_eq!(receivers.len(), 3);
    }

    #[test]
    fn test_exchange_with_exclusions() {
        let participants = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Carol".to_string(),
            "Dave".to_string(),
        ];
        let exclusions = vec![("Alice".to_string(), "Bob".to_string())];

        let result = build_exchange(&participants, &exclusions);
        assert!(result.is_some());

        let pairings = result.unwrap();
        for (giver, receiver) in &pairings {
            if giver == "Alice" {
                assert_ne!(receiver, "Bob");
            }
            if giver == "Bob" {
                assert_ne!(receiver, "Alice");
            }
        }
    }

    #[test]
    fn test_select_letter() {
        let excluded = vec!['A', 'B', 'C'];
        let letter = select_letter(&excluded);
        assert!(letter.is_some());
        assert!(!excluded.contains(&letter.unwrap()));
    }

    #[test]
    fn test_select_letter_all_excluded() {
        let excluded: Vec<char> = ('A'..='Z').collect();
        let letter = select_letter(&excluded);
        assert!(letter.is_none());
    }
}
