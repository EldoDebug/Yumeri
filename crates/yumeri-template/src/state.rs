use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default)]
pub struct StateSet(pub HashSet<String>);

impl StateSet {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn insert(&mut self, state: impl Into<String>) {
        self.0.insert(state.into());
    }

    pub fn contains(&self, state: &str) -> bool {
        self.0.contains(state)
    }

    /// Find the best matching state key from a map of state overrides.
    /// Keys can be compound (e.g. "checked+hovered"). The most specific match
    /// (most parts) wins. Returns matching keys sorted by specificity (most specific first).
    pub fn find_matching_states<'a, V>(
        &self,
        states_map: &'a HashMap<String, V>,
    ) -> Vec<&'a str> {
        let mut matches: Vec<(&str, usize)> = states_map
            .keys()
            .filter_map(|key| {
                if key == "default" {
                    return Some((key.as_str(), 0));
                }
                let parts: Vec<&str> = key.split('+').collect();
                let all_match = parts.iter().all(|part| self.0.contains(*part));
                if all_match {
                    Some((key.as_str(), parts.len()))
                } else {
                    None
                }
            })
            .collect();

        // Sort by specificity (most parts first), "default" last
        matches.sort_by(|a, b| b.1.cmp(&a.1));
        matches.into_iter().map(|(k, _)| k).collect()
    }
}
