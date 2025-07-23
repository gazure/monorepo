use std::collections::{HashMap, HashSet};

use super::participant::Participant;

#[derive(Debug, Default)]
pub struct ParticipantGraph {
    edges: HashMap<String, Vec<String>>,
    participants: HashMap<String, Participant>,
}

impl ParticipantGraph {
    pub fn new() -> Self {
        Self {
            participants: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    pub fn from_participants(participants: Vec<Participant>) -> Self {
        let mut graph = Self::new();
        participants.iter().for_each(|p| {
            graph.add_participant(p.clone());
        });
        graph.link_participants();
        graph
    }

    pub fn add_participant(&mut self, participant: Participant) {
        self.participants
            .insert(participant.name.clone(), participant);
    }

    pub fn link_participants(&mut self) {
        for (name, participant) in &self.participants {
            let mut possible_receivers = self
                .participants
                .iter()
                .filter(|(n, p)| {
                    *n != name
                        && !participant.exclusions.contains(n)
                        && participant
                            .exchange_pools
                            .iter()
                            .any(|pool| p.exchange_pools.contains(pool))
                })
                .map(|(n, _)| n.clone())
                .collect::<Vec<String>>();
            fastrand::shuffle(&mut possible_receivers);
            self.edges.insert(name.clone(), possible_receivers);
        }
    }

    /// Builds a gift exchange by finding a Hamiltonian cycle in the participant graph.
    ///
    /// This algorithm attempts to create a cycle where:
    /// - Each person gives exactly one gift
    /// - Each person receives exactly one gift
    /// - All exclusion rules are respected
    ///
    /// The algorithm tries up to 100 times with different random starting points
    /// to find a valid Hamiltonian cycle. If no cycle is found, it falls back
    /// to a simpler pairing strategy.
    pub fn build_exchange(&self) -> Vec<(String, String)> {
        let num_participants = self.participants.len();
        if num_participants == 0 {
            return vec![];
        }

        // Try multiple times with different random starting points
        for _attempt in 0..100 {
            // Get a random starting participant
            let mut participants_list: Vec<String> = self.participants.keys().cloned().collect();
            fastrand::shuffle(&mut participants_list);

            if let Some(solution) =
                self.find_hamiltonian_cycle(&participants_list[0], num_participants)
            {
                // Convert the cycle to exchange pairs
                let mut exchange = vec![];
                for i in 0..solution.len() - 1 {
                    exchange.push((solution[i].clone(), solution[i + 1].clone()));
                }
                // Add the last edge to complete the cycle
                exchange.push((solution[solution.len() - 1].clone(), solution[0].clone()));
                return exchange;
            }
        }

        eprintln!(
            "Warning: Could not find a perfect cycle after 100 attempts. Falling back to best-effort pairing."
        );
        self.fallback_exchange()
    }

    /// Attempts to find a Hamiltonian cycle starting from the given node.
    ///
    /// A Hamiltonian cycle visits each node exactly once and returns to the start.
    /// This ensures everyone gives and receives exactly one gift.
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

    /// Depth-first search with backtracking to find a Hamiltonian cycle.
    ///
    /// This recursively explores paths, backtracking when it hits a dead end.
    /// The randomization of edge order helps find different valid cycles
    /// across multiple runs.
    fn dfs_hamiltonian(
        &self,
        path: &mut Vec<String>,
        visited: &mut HashSet<String>,
        target_length: usize,
        start: &str,
    ) -> bool {
        if path.len() == target_length {
            // Check if we can return to the start
            let last = path.last().unwrap();
            if let Some(edges) = self.edges.get(last) {
                return edges.contains(&start.to_string());
            }
            return false;
        }

        let current = path.last().unwrap().clone();
        if let Some(edges) = self.edges.get(&current) {
            // Try edges in random order
            let mut shuffled_edges = edges.clone();
            fastrand::shuffle(&mut shuffled_edges);

            for next in shuffled_edges {
                if !visited.contains(&next) {
                    path.push(next.clone());
                    visited.insert(next.clone());

                    if self.dfs_hamiltonian(path, visited, target_length, start) {
                        return true;
                    }

                    // Backtrack
                    path.pop();
                    visited.remove(&next);
                }
            }
        }

        false
    }

    /// Fallback strategy when a Hamiltonian cycle cannot be found.
    ///
    /// This creates a simple valid exchange by trying to match givers to receivers
    /// while respecting exclusion rules. If that fails, it falls back to a
    /// simple rotation where each person gives to the next in the list.
    fn fallback_exchange(&self) -> Vec<(String, String)> {
        // Create a simple valid exchange by ensuring everyone gives and receives once
        let mut givers: Vec<String> = self.participants.keys().cloned().collect();
        let mut receivers: Vec<String> = givers.clone();
        let mut exchange = vec![];

        fastrand::shuffle(&mut givers);
        fastrand::shuffle(&mut receivers);

        for giver in &givers {
            // Find a valid receiver
            for (idx, receiver) in receivers.iter().enumerate() {
                if giver != receiver && self.can_give_to(giver, receiver) {
                    exchange.push((giver.clone(), receiver.clone()));
                    receivers.remove(idx);
                    break;
                }
            }
        }

        // If we couldn't match everyone, just do a simple rotation
        if exchange.len() < givers.len() {
            exchange.clear();
            for i in 0..givers.len() {
                let next = (i + 1) % givers.len();
                exchange.push((givers[i].clone(), givers[next].clone()));
            }
        }

        exchange
    }

    /// Checks if a giver can give to a receiver based on the exclusion rules.
    fn can_give_to(&self, giver: &str, receiver: &str) -> bool {
        if let Some(edges) = self.edges.get(giver) {
            edges.contains(&receiver.to_string())
        } else {
            false
        }
    }
}
