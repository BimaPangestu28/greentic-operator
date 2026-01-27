use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::Value;

use messaging_cardkit::{CardKit, StaticProfiles, Tier};

/// Lightweight renderer that applies messaging-cardkit before packs receive payloads.
#[derive(Clone)]
pub struct CardRenderer {
    kit: Arc<CardKit<StaticProfiles>>,
}

/// Metadata returned when rendering occurs.
#[derive(Debug, Clone)]
pub struct RenderMetadata {
    pub tier: Tier,
    pub target_tier: Tier,
    pub downgraded: bool,
    pub warnings_count: usize,
}

/// Outcome from attempting to render a provider card.
pub struct RenderOutcome {
    pub bytes: Vec<u8>,
    pub metadata: Option<RenderMetadata>,
}

impl CardRenderer {
    /// Create a renderer that downgrades to the given tier by default.
    pub fn new(default_tier: Tier) -> Self {
        let profiles = Arc::new(StaticProfiles::builder().default_tier(default_tier).build());
        let kit = CardKit::new(profiles);
        Self { kit: Arc::new(kit) }
    }

    /// Render the payload if it contains a `message_card`/`card` entry.
    pub fn render_if_needed(
        &self,
        provider_type: &str,
        payload_bytes: &[u8],
    ) -> Result<RenderOutcome> {
        let mut payload: Value = match serde_json::from_slice(payload_bytes) {
            Ok(value) => value,
            Err(_) => return Ok(RenderOutcome::unchanged(payload_bytes.to_vec())),
        };
        let card_value = match find_card_value(&mut payload) {
            Some(card) => card,
            None => return Ok(RenderOutcome::unchanged(payload_bytes.to_vec())),
        };
        let response = self
            .kit
            .render(provider_type, card_value)
            .context("card rendering failed")?;
        *card_value = response.payload.clone();
        let bytes = serde_json::to_vec(&payload)?;
        Ok(RenderOutcome {
            bytes,
            metadata: Some(RenderMetadata {
                tier: response.preview.tier,
                target_tier: response.preview.target_tier,
                downgraded: response.downgraded,
                warnings_count: response.warnings.len(),
            }),
        })
    }
}

impl RenderOutcome {
    fn unchanged(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            metadata: None,
        }
    }
}

fn find_card_value(payload: &mut Value) -> Option<&mut Value> {
    const PATHS: &[&[&str]] = &[
        &["message_card"],
        &["card"],
        &["payload", "message_card"],
        &["payload", "card"],
    ];
    let path = PATHS
        .iter()
        .find(|path| payload_contains_path(payload, path))?;
    get_mut_by_path(payload, path)
}

fn payload_contains_path(payload: &Value, path: &[&str]) -> bool {
    let mut current = payload;
    for key in path {
        match current {
            Value::Object(map) => {
                current = match map.get(*key) {
                    Some(value) => value,
                    None => return false,
                };
            }
            _ => return false,
        }
    }
    current.is_object()
}

fn get_mut_by_path<'a>(value: &'a mut Value, path: &[&str]) -> Option<&'a mut Value> {
    let mut current = value;
    for key in path {
        match current {
            Value::Object(map) => {
                current = map.get_mut(*key)?;
            }
            _ => return None,
        }
    }
    Some(current)
}
