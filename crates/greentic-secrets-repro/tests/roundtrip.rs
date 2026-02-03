use std::str;

use anyhow::{anyhow, Result};
use greentic_secrets_lib::{
    apply_seed, ApplyOptions, DevStore, SecretFormat, SecretsStore, SeedDoc, SeedEntry, SeedValue,
};
use tempfile::tempdir;
use tracing::debug;

use greentic_secrets_repro::{canonical_secret_uri, init_tracing};
use uuid::Uuid;

const ENV: &str = "demo";
const TENANT_ACME: &str = "acme";
const TENANT_3POINT: &str = "3point";
const TEAM: &str = "core";
const PROVIDER_TELEGRAM: &str = "messaging-telegram";
const PROVIDER_WEBEX: &str = "messaging-webex";
const TELEGRAM_KEY: &str = "telegram_bot_token";
const WEBEX_KEY: &str = "webex_bot_token";

#[tokio::test]
async fn roundtrip_and_persistence() -> Result<()> {
    init_tracing();
    let dir = tempdir()?;
    let store_path = dir.path().join("dev-store.env");
    debug!(backend = "dev-store", path = ?store_path, "creating dev store backend");

    let store = DevStore::with_path(&store_path).map_err(|err| {
        anyhow!(
            "failed to open dev secrets store {}: {err}",
            store_path.display()
        )
    })?;

    let uri_telegram =
        canonical_secret_uri(ENV, TENANT_ACME, None, PROVIDER_TELEGRAM, TELEGRAM_KEY);
    log_canonicalization(
        ENV,
        TENANT_ACME,
        None,
        PROVIDER_TELEGRAM,
        TELEGRAM_KEY,
        &uri_telegram,
    );
    let uri_webex = canonical_secret_uri(ENV, TENANT_3POINT, Some(TEAM), PROVIDER_WEBEX, WEBEX_KEY);
    log_canonicalization(
        ENV,
        TENANT_3POINT,
        Some(TEAM),
        PROVIDER_WEBEX,
        WEBEX_KEY,
        &uri_webex,
    );

    let secret_telegram = random_secret();
    let secret_webex = random_secret();
    let telegram_bytes = secret_telegram.as_bytes().to_vec();
    let webex_bytes = secret_webex.as_bytes().to_vec();
    let seed = SeedDoc {
        entries: vec![
            make_seed_entry(uri_telegram.clone(), &secret_telegram),
            make_seed_entry(uri_webex.clone(), &secret_webex),
        ],
    };

    let report = apply_seed(&store, &seed, ApplyOptions::default()).await;
    assert_eq!(report.ok, 2);
    assert!(report.failed.is_empty(), "apply_seed failed: {report:?}");

    let telegram_value = store.get(&uri_telegram).await?;
    debug!(uri = %uri_telegram, len = telegram_value.len(), "retrieved telegram secret");
    assert_eq!(telegram_value, telegram_bytes);
    let webex_value = store.get(&uri_webex).await?;
    debug!(uri = %uri_webex, len = webex_value.len(), "retrieved webex secret");
    assert_eq!(webex_value, webex_bytes);

    drop(store);
    debug!(path = ?store_path, "reopening dev store backend for persistence check");
    let reopened_store = DevStore::with_path(&store_path).map_err(|err| {
        anyhow!(
            "failed to reopen dev secrets store {}: {err}",
            store_path.display()
        )
    })?;
    let persisted = reopened_store.get(&uri_telegram).await?;
    assert_eq!(persisted, telegram_bytes);

    Ok(())
}

#[tokio::test]
async fn missing_secret_reports_canonical_key() -> Result<()> {
    init_tracing();
    let dir = tempdir()?;
    let store_path = dir.path().join("dev-store.env");
    let store = DevStore::with_path(&store_path)?;

    let missing_uri =
        canonical_secret_uri(ENV, TENANT_ACME, None, PROVIDER_TELEGRAM, "missing_secret");
    log_canonicalization(
        ENV,
        TENANT_ACME,
        None,
        PROVIDER_TELEGRAM,
        "missing_secret",
        &missing_uri,
    );
    let err = store
        .get(&missing_uri)
        .await
        .expect_err("secret should be missing");
    let msg = err.to_string();
    debug!(uri = %missing_uri, error = %msg, "missing secret error");
    assert!(
        msg.contains(&missing_uri),
        "expected error to mention canonical URI, got {msg}"
    );
    Ok(())
}

fn log_canonicalization(
    env: &str,
    tenant: &str,
    team: Option<&str>,
    provider: &str,
    key: &str,
    uri: &str,
) {
    let team_display = team.unwrap_or("default");
    debug!(
        env = %env,
        tenant = %tenant,
        team = %team_display,
        provider = %provider,
        key = %key,
        uri = %uri,
        "canonicalized secret URI"
    );
}

fn make_seed_entry(uri: String, value: &str) -> SeedEntry {
    SeedEntry {
        uri,
        format: SecretFormat::Text,
        value: SeedValue::Text {
            text: value.to_string(),
        },
        description: Some("repro test secret".to_string()),
    }
}

fn random_secret() -> String {
    Uuid::new_v4().simple().to_string()
}
