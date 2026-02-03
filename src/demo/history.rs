use crate::demo::card::CardView;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Snapshot {
    pub output: JsonValue,
    pub card: Option<CardView>,
    pub pending_inputs: HashMap<String, String>,
}

impl Snapshot {
    pub fn new(
        output: JsonValue,
        card: Option<CardView>,
        pending_inputs: HashMap<String, String>,
    ) -> Self {
        Self {
            output,
            card,
            pending_inputs,
        }
    }
}

pub struct DemoHistory {
    stack: Vec<Snapshot>,
}

impl DemoHistory {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self, snapshot: Snapshot) {
        self.stack.push(snapshot);
    }

    pub fn latest(&self) -> Option<&Snapshot> {
        self.stack.last()
    }

    pub fn go_back(&mut self) -> Option<&Snapshot> {
        if self.stack.len() <= 1 {
            return None;
        }
        self.stack.pop();
        self.stack.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_snapshot(version: &str, inputs: &[(&str, &str)]) -> Snapshot {
        let card = CardView {
            version: Some(version.to_string()),
            title: Some(format!("Card {version}")),
            summary_text: None,
            body_texts: Vec::new(),
            inputs: Vec::new(),
            actions: Vec::new(),
        };
        let pending = inputs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Snapshot::new(
            json!({ "card": { "version": version } }),
            Some(card),
            pending,
        )
    }

    #[test]
    fn go_back_restores_previous_snapshot() {
        let mut history = DemoHistory::new();
        history.push(make_snapshot("1.0", &[("foo", "bar")]));
        history.push(make_snapshot("2.0", &[("foo", "baz")]));
        let previous = history.go_back().expect("should be able to go back");
        assert_eq!(
            previous.card.as_ref().unwrap().version.as_deref(),
            Some("1.0")
        );
        assert_eq!(
            previous.pending_inputs.get("foo").map(String::as_str),
            Some("bar")
        );
    }

    #[test]
    fn cannot_go_back_past_first_snapshot() {
        let mut history = DemoHistory::new();
        history.push(make_snapshot("1.0", &[("foo", "bar")]));
        assert!(history.go_back().is_none());
    }
}
