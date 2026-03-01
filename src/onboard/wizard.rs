use http_body_util::Full;
use hyper::{Response, StatusCode, body::Bytes};
use serde_json::{Value, json};

use crate::component_qa_ops::{self, QaMode};
use crate::domains::{self, Domain, ProviderPack};
use crate::gmap;
use crate::operator_log;
use crate::qa_persist;
use crate::setup_to_formspec;

use qa_spec::{build_render_payload, render_json_ui};

use super::api::{OnboardState, error_response, json_ok};

/// POST /api/onboard/qa/spec
///
/// Returns the FormSpec JSON UI for a provider.
///
/// Request body: `{ "provider_id": "messaging-telegram", "domain": "messaging",
///                  "tenant": "default", "team": null, "mode": "setup",
///                  "answers": {} }`
pub fn get_form_spec(
    state: &OnboardState,
    body: &Value,
) -> Result<Response<Full<Bytes>>, Response<Full<Bytes>>> {
    let provider_id = body["provider_id"]
        .as_str()
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "missing provider_id"))?;
    let domain = parse_domain(body)?;
    let tenant_owned = body["tenant"].as_str().unwrap_or("default").to_ascii_lowercase();
    let tenant = tenant_owned.as_str();
    let team_owned = body["team"].as_str().map(|s| s.to_ascii_lowercase());
    let team = team_owned.as_deref();
    let answers = body.get("answers").cloned().unwrap_or_else(|| json!({}));

    let locale = body["locale"].as_str().unwrap_or("en");
    let mode = parse_mode(body);

    let bundle_root = state.runner_host.bundle_root();
    let pack = find_provider_pack(bundle_root, domain, provider_id)?;

    // Try WASM qa-spec first, fall back to setup.yaml → FormSpec
    let form_spec = match get_form_spec_from_pack(bundle_root, domain, &pack, provider_id, tenant, team, locale, mode) {
        Some(spec) => {
            operator_log::info(
                module_path!(),
                format!("[onboard] qa/spec path=wasm provider={} questions={}", provider_id, spec.questions.len()),
            );
            spec
        }
        None => {
            operator_log::info(
                module_path!(),
                format!("[onboard] qa/spec path=fallback provider={} pack={}", provider_id, pack.path.display()),
            );
            // Fallback: try setup.yaml conversion + apply i18n from disk
            let mut spec = setup_to_formspec::pack_to_form_spec(&pack.path, provider_id).ok_or_else(|| {
                error_response(
                    StatusCode::NOT_FOUND,
                    format!("no qa-spec or setup.yaml found in {}", pack.file_name),
                )
            })?;
            apply_i18n_to_form_spec(&mut spec, bundle_root, provider_id, locale, mode.as_str());
            spec
        }
    };

    // For upgrade mode: pre-fill answers with existing config values
    let answers = if mode == QaMode::Upgrade {
        merge_existing_config(bundle_root, provider_id, &answers)
    } else {
        answers
    };

    let ctx = json!({ "tenant": tenant, "team": team });
    let payload = build_render_payload(&form_spec, &ctx, &answers);
    let rendered = render_json_ui(&payload);

    operator_log::info(
        module_path!(),
        format!(
            "[onboard] qa/spec provider={} status={}",
            provider_id,
            payload.status.as_str()
        ),
    );

    let mut response = rendered;
    if let Some(url) = read_runtime_public_url(bundle_root, tenant, team) {
        response["meta"] = json!({ "public_url": url });
    }

    json_ok(response)
}

/// POST /api/onboard/qa/validate
///
/// Validates partial answers and returns updated progress.
///
/// Request body: `{ "provider_id": "messaging-telegram", "domain": "messaging",
///                  "tenant": "default", "answers": { ... } }`
pub fn validate_answers(
    state: &OnboardState,
    body: &Value,
) -> Result<Response<Full<Bytes>>, Response<Full<Bytes>>> {
    let provider_id = body["provider_id"]
        .as_str()
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "missing provider_id"))?;
    let domain = parse_domain(body)?;
    let tenant_owned = body["tenant"].as_str().unwrap_or("default").to_ascii_lowercase();
    let tenant = tenant_owned.as_str();
    let team_owned = body["team"].as_str().map(|s| s.to_ascii_lowercase());
    let team = team_owned.as_deref();
    let answers = body.get("answers").cloned().unwrap_or_else(|| json!({}));
    let locale = body["locale"].as_str().unwrap_or("en");
    let mode = parse_mode(body);

    let bundle_root = state.runner_host.bundle_root();
    let pack = find_provider_pack(bundle_root, domain, provider_id)?;

    let form_spec = match get_form_spec_from_pack(bundle_root, domain, &pack, provider_id, tenant, team, locale, mode) {
        Some(spec) => spec,
        None => {
            let mut spec = setup_to_formspec::pack_to_form_spec(&pack.path, provider_id).ok_or_else(|| {
                error_response(
                    StatusCode::NOT_FOUND,
                    format!("no qa-spec found for {}", provider_id),
                )
            })?;
            apply_i18n_to_form_spec(&mut spec, bundle_root, provider_id, locale, mode.as_str());
            spec
        }
    };

    let ctx = json!({ "tenant": tenant, "team": team });
    let payload = build_render_payload(&form_spec, &ctx, &answers);
    let rendered = render_json_ui(&payload);

    let mut response = rendered;
    if let Some(url) = read_runtime_public_url(bundle_root, tenant, team) {
        response["meta"] = json!({ "public_url": url });
    }

    json_ok(response)
}

/// POST /api/onboard/qa/submit
///
/// Submits answers, persists secrets + config, and updates gmap.
///
/// Request body: `{ "provider_id": "messaging-telegram", "domain": "messaging",
///                  "tenant": "default", "team": null, "answers": { ... } }`
pub fn submit_answers(
    state: &OnboardState,
    body: &Value,
) -> Result<Response<Full<Bytes>>, Response<Full<Bytes>>> {
    let provider_id = body["provider_id"]
        .as_str()
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "missing provider_id"))?;
    let domain = parse_domain(body)?;
    let tenant_owned = body["tenant"].as_str().unwrap_or("default").to_ascii_lowercase();
    let tenant = tenant_owned.as_str();
    let team_owned = body["team"].as_str().map(|s| s.to_ascii_lowercase());
    let team = team_owned.as_deref();
    let answers = body.get("answers").cloned().unwrap_or_else(|| json!({}));

    let bundle_root = state.runner_host.bundle_root();
    let pack = find_provider_pack(bundle_root, domain, provider_id)?;

    operator_log::info(
        module_path!(),
        format!(
            "[onboard] qa/submit provider={} tenant={} team={:?}",
            provider_id, tenant, team
        ),
    );

    // 1. Run apply-answers via component QA
    let mode = parse_mode(body);
    let current_config = if mode == QaMode::Upgrade || mode == QaMode::Remove {
        crate::provider_config_envelope::read_provider_config_envelope(
            &bundle_root.join(".providers"),
            provider_id,
        )
        .ok()
        .flatten()
        .map(|envelope| envelope.config)
    } else {
        None
    };
    let config = component_qa_ops::apply_answers_via_component_qa(
        bundle_root,
        domain,
        tenant,
        team,
        &pack,
        provider_id,
        mode,
        current_config.as_ref(),
        &answers,
    )
    .map_err(|err| {
        operator_log::error(
            module_path!(),
            format!("[onboard] apply-answers failed: {err}"),
        );
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("apply-answers failed: {err}"),
        )
    })?;

    let mut config = match config {
        Some(config) => config,
        None => {
            // No WASM QA contract — use answers directly as config
            answers.clone()
        }
    };

    // Re-inject UI-level fields that WASM may have stripped
    if let Some(map) = config.as_object_mut() {
        // instance_label: user-friendly name for the provider instance
        if let Some(label) = answers.get("instance_label").and_then(Value::as_str) {
            if !label.is_empty() {
                map.insert("instance_label".to_string(), Value::String(label.to_string()));
            }
        }
        // Persist deployment scope so upgrade can restore it
        map.insert("_scope_tenant".to_string(), Value::String(tenant.to_string()));
        if let Some(t) = team {
            map.insert("_scope_team".to_string(), Value::String(t.to_string()));
        }
    }

    // 2. Get FormSpec (for secret field identification — locale not needed here)
    let mut form_spec = match get_form_spec_from_pack(bundle_root, domain, &pack, provider_id, tenant, team, "en", mode) {
        Some(spec) => spec,
        None => setup_to_formspec::pack_to_form_spec(&pack.path, provider_id)
            .unwrap_or_else(|| make_minimal_form_spec(provider_id, &config)),
    };

    // 3. Inject secret aliases into config + FormSpec so they're persisted in the same batch
    //    (avoids DEK cache bug from separate DevStore instances)
    if provider_id == "messaging-telegram" {
        if let Some(token) = config.get("bot_token").and_then(Value::as_str).map(String::from) {
            if !token.is_empty() {
                if let Some(map) = config.as_object_mut() {
                    map.entry("telegram_bot_token".to_string())
                        .or_insert_with(|| Value::String(token));
                }
                // Add synthetic secret question so persist picks it up
                if !form_spec.questions.iter().any(|q| q.id == "telegram_bot_token") {
                    form_spec.questions.push(qa_spec::QuestionSpec {
                        id: "telegram_bot_token".to_string(),
                        kind: qa_spec::QuestionType::String,
                        title: "telegram_bot_token".to_string(),
                        title_i18n: None,
                        description: None,
                        description_i18n: None,
                        required: false,
                        choices: None,
                        default_value: None,
                        secret: true,
                        visible_if: None,
                        constraint: None,
                        list: None,
                        computed: None,
                        policy: Default::default(),
                        computed_overridable: false,
                    });
                }
            }
        }
    }

    // Persist secrets + config (single DevStore instance writes all secrets in one batch)
    let providers_root = bundle_root.join(".providers");
    let rt = tokio::runtime::Runtime::new().expect("persist runtime");
    let persist_result = rt
        .block_on(qa_persist::persist_qa_results(
            bundle_root,
            &providers_root,
            tenant,
            team,
            provider_id,
            &config,
            &pack.path,
            &form_spec,
            true,
        ))
        .map_err(|err| {
        operator_log::error(
            module_path!(),
            format!("[onboard] persist failed: {err}"),
        );
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("persist failed: {err}"),
        )
    })?;
    let (secrets_saved, config_written) = persist_result;

    // 4. Update gmap policy
    let gmap_path = resolve_gmap_path(bundle_root, tenant, team);
    let rule_path = format!("{provider_id}");
    if let Err(err) = gmap::upsert_policy(&gmap_path, &rule_path, gmap::Policy::Public) {
        operator_log::error(
            module_path!(),
            format!("[onboard] gmap upsert failed: {err}"),
        );
        return Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("gmap update failed: {err}"),
        ));
    }

    if mode == QaMode::Remove {
        // For remove: delete the provider config and revoke gmap access
        let provider_dir = bundle_root.join(".providers").join(provider_id);
        if provider_dir.exists() {
            let _ = std::fs::remove_dir_all(&provider_dir);
        }
        if let Err(err) = gmap::upsert_policy(&gmap_path, &rule_path, gmap::Policy::Forbidden) {
            operator_log::error(
                module_path!(),
                format!("[onboard] gmap revoke failed: {err}"),
            );
        }
    }

    // 5. Run setup flows from pack (skip for remove mode)
    let mut setup_flow_result: Option<Value> = None;
    let mut verify_flow_result: Option<Value> = None;
    let webhook_result;

    // Inject runtime-detected public URL into config if not already set
    let runtime_url = read_runtime_public_url(bundle_root, tenant, team);
    if let Some(ref url) = runtime_url {
        if let Some(map) = config.as_object_mut() {
            map.entry("public_base_url".to_string())
                .or_insert_with(|| Value::String(url.clone()));
        }
    }

    if mode != QaMode::Remove {
        let has_setup_flow = pack.entry_flows.iter().any(|f| f == "setup_default");

        if has_setup_flow {
            // Build input payload for the setup flow (mirrors providers.rs build_input)
            let public_base_url = config.get("public_base_url").and_then(Value::as_str);
            let flow_input = build_setup_flow_input(provider_id, tenant, team, public_base_url, &config);
            let payload_bytes = serde_json::to_vec(&flow_input).unwrap_or_default();

            let ctx = crate::demo::runner_host::OperatorContext {
                tenant: tenant.to_string(),
                team: team.map(|t| t.to_string()),
                correlation_id: None,
            };

            operator_log::info(
                module_path!(),
                format!("[onboard] running setup_default flow for {}", provider_id),
            );

            match state.runner_host.invoke_provider_op(domain, provider_id, "setup_default", &payload_bytes, &ctx) {
                Ok(outcome) => {
                    operator_log::info(
                        module_path!(),
                        format!(
                            "[onboard] setup_default flow complete provider={} success={} error={:?}",
                            provider_id, outcome.success, outcome.error
                        ),
                    );
                    setup_flow_result = Some(json!({
                        "flow": "setup_default",
                        "success": outcome.success,
                        "error": outcome.error,
                        "output": outcome.output,
                    }));
                }
                Err(err) => {
                    operator_log::error(
                        module_path!(),
                        format!("[onboard] setup_default flow failed for {}: {err}", provider_id),
                    );
                    setup_flow_result = Some(json!({
                        "flow": "setup_default",
                        "success": false,
                        "error": err.to_string(),
                    }));
                }
            }

            // Run verify_webhooks if available
            let has_verify_flow = pack.entry_flows.iter().any(|f| f == "verify_webhooks");
            if has_verify_flow {
                operator_log::info(
                    module_path!(),
                    format!("[onboard] running verify_webhooks flow for {}", provider_id),
                );

                match state.runner_host.invoke_provider_op(domain, provider_id, "verify_webhooks", &payload_bytes, &ctx) {
                    Ok(outcome) => {
                        operator_log::info(
                            module_path!(),
                            format!(
                                "[onboard] verify_webhooks flow complete provider={} success={}",
                                provider_id, outcome.success
                            ),
                        );
                        verify_flow_result = Some(json!({
                            "flow": "verify_webhooks",
                            "success": outcome.success,
                            "error": outcome.error,
                            "output": outcome.output,
                        }));
                    }
                    Err(err) => {
                        operator_log::warn(
                            module_path!(),
                            format!("[onboard] verify_webhooks flow failed for {}: {err}", provider_id),
                        );
                        verify_flow_result = Some(json!({
                            "flow": "verify_webhooks",
                            "success": false,
                            "error": err.to_string(),
                        }));
                    }
                }
            }

            // Also call native webhook setup (WASM flows are templates, not actual API calls)
            webhook_result = try_provider_setup_webhook(
                bundle_root, domain, &pack, provider_id, tenant, team, &config,
            );
        } else {
            // No setup flow in pack — fall back to manual webhook setup
            webhook_result = try_provider_setup_webhook(
                bundle_root, domain, &pack, provider_id, tenant, team, &config,
            );
        }
    } else {
        webhook_result = None;
    }

    if let Some(ref result) = webhook_result {
        operator_log::info(
            module_path!(),
            format!("[onboard] setup_webhook provider={} result={}", provider_id, result),
        );
    }

    operator_log::info(
        module_path!(),
        format!(
            "[onboard] qa/submit complete provider={} secrets={} config={}",
            provider_id,
            secrets_saved.len(),
            config_written
        ),
    );

    json_ok(json!({
        "status": "ok",
        "provider_id": provider_id,
        "mode": mode.as_str(),
        "secrets_saved": secrets_saved,
        "config_written": config_written,
        "gmap_updated": true,
        "webhook_setup": webhook_result,
        "setup_flow": setup_flow_result,
        "verify_flow": verify_flow_result,
    }))
}

/// Try to get a FormSpec from the WASM qa-spec op.
fn get_form_spec_from_pack(
    bundle_root: &std::path::Path,
    domain: Domain,
    pack: &ProviderPack,
    provider_id: &str,
    tenant: &str,
    team: Option<&str>,
    locale: &str,
    mode: QaMode,
) -> Option<qa_spec::FormSpec> {
    use crate::demo::qa_bridge;
    use crate::demo::runner_host::{DemoRunnerHost, OperatorContext};
    use crate::discovery::{self, DiscoveryOptions};
    use crate::secrets_gate;
    use super::provider_i18n;

    let cbor_only = bundle_root.join("greentic.demo.yaml").exists();
    let discovery = discovery::discover_with_options(bundle_root, DiscoveryOptions { cbor_only }).ok()?;
    let secrets_handle = secrets_gate::resolve_secrets_manager(bundle_root, tenant, team).ok()?;
    let host = DemoRunnerHost::new(
        bundle_root.to_path_buf(),
        &discovery,
        None,
        secrets_handle,
        false,
    )
    .ok()?;

    let ctx = OperatorContext {
        tenant: tenant.to_string(),
        team: team.map(|t| t.to_string()),
        correlation_id: None,
    };

    // 1. Invoke qa-spec via schema-core-api invoke()
    let qa_payload = serde_json::to_vec(&json!({"mode": mode.as_str()})).ok()?;
    let qa_out = match host
        .invoke_provider_component_op_direct(domain, pack, provider_id, "qa-spec", &qa_payload, &ctx)
    {
        Ok(out) => out,
        Err(err) => {
            operator_log::info(
                module_path!(),
                format!("[onboard] qa-spec invoke failed for {}: {}", provider_id, err),
            );
            return None;
        }
    };

    if !qa_out.success {
        operator_log::info(
            module_path!(),
            format!("[onboard] qa-spec not successful for {}: {:?}", provider_id, qa_out.error),
        );
        return None;
    }
    let qa_json = match qa_out.output {
        Some(json) => json,
        None => {
            operator_log::info(
                module_path!(),
                format!("[onboard] qa-spec output is None for {}", provider_id),
            );
            return None;
        }
    };

    // 2. Fetch English i18n from WASM component (always English)
    let wasm_english: std::collections::BTreeMap<String, String> =
        fetch_i18n_bundle(&host, domain, pack, provider_id, &ctx)
            .into_iter()
            .collect();

    // 3. Load disk files + merge via qa_spec::merge_i18n_layers
    let i18n_dir = provider_i18n::resolve_i18n_dir(bundle_root);
    operator_log::info(
        module_path!(),
        format!(
            "[onboard] i18n: bundle_root={} locale={} dir={:?} wasm_keys={}",
            bundle_root.display(),
            locale,
            i18n_dir,
            wasm_english.len(),
        ),
    );
    let merged = provider_i18n::load_and_merge(&wasm_english, locale, i18n_dir.as_deref());
    operator_log::info(
        module_path!(),
        format!("[onboard] i18n: merged_keys={}", merged.len()),
    );

    // 4. Use qa_bridge which resolves i18n titles and descriptions
    let i18n_map: std::collections::HashMap<String, String> = merged.into_iter().collect();
    let form_spec = qa_bridge::provider_qa_to_form_spec(&qa_json, &i18n_map, provider_id);

    Some(form_spec)
}

/// Fetch i18n translations from the WASM component via the `i18n-bundle` op.
///
/// The provider's `schema-core-api::invoke("i18n-bundle", locale)` returns
/// `{"locale":"en","messages":{"key":"translation",...}}`.
fn fetch_i18n_bundle(
    host: &crate::demo::runner_host::DemoRunnerHost,
    domain: Domain,
    pack: &ProviderPack,
    provider_id: &str,
    ctx: &crate::demo::runner_host::OperatorContext,
) -> std::collections::HashMap<String, String> {
    let locale_payload = serde_json::to_vec(&json!("en")).unwrap_or_default();
    let bundle_out = match host.invoke_provider_component_op_direct(
        domain,
        pack,
        provider_id,
        "i18n-bundle",
        &locale_payload,
        ctx,
    ) {
        Ok(out) if out.success => out,
        Ok(out) => {
            operator_log::info(
                module_path!(),
                format!(
                    "[onboard] i18n-bundle failed for {}: {:?}",
                    provider_id, out.error
                ),
            );
            return std::collections::HashMap::new();
        }
        Err(err) => {
            operator_log::info(
                module_path!(),
                format!("[onboard] i18n-bundle error for {}: {}", provider_id, err),
            );
            return std::collections::HashMap::new();
        }
    };

    let Some(bundle_json) = bundle_out.output else {
        return std::collections::HashMap::new();
    };

    // Parse {"locale":"en","messages":{"key":"value",...}}
    bundle_json
        .get("messages")
        .and_then(Value::as_object)
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn make_minimal_form_spec(provider_id: &str, config: &Value) -> qa_spec::FormSpec {
    use qa_spec::{FormSpec, QuestionSpec};

    let questions = config
        .as_object()
        .map(|map| {
            map.keys()
                .map(|key| {
                    let (kind, secret, _) = setup_to_formspec::infer_question_properties(key);
                    QuestionSpec {
                        id: key.clone(),
                        kind,
                        title: key.clone(),
                        title_i18n: None,
                        description: None,
                        description_i18n: None,
                        required: false,
                        choices: None,
                        default_value: None,
                        secret,
                        visible_if: None,
                        constraint: None,
                        list: None,
                        computed: None,
                        policy: Default::default(),
                        computed_overridable: false,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    FormSpec {
        id: format!("{provider_id}-setup"),
        title: format!("{provider_id} setup"),
        version: "1.0.0".to_string(),
        description: None,
        presentation: None,
        progress_policy: None,
        secrets_policy: None,
        store: vec![],
        validations: vec![],
        includes: vec![],
        questions,
    }
}

/// Apply i18n translations from disk locale files to a FormSpec produced by
/// the setup.yaml fallback path.
///
/// This resolves titles and descriptions using the same i18n key conventions
/// as the WASM qa-spec path:
///   - title:       `{provider}.qa.{mode}.{field_id}`
///   - description: `{provider}.schema.config.{field_id}.description`
///   - form title:  `{provider}.qa.{mode}.title`
fn apply_i18n_to_form_spec(
    form_spec: &mut qa_spec::FormSpec,
    bundle_root: &std::path::Path,
    provider_id: &str,
    locale: &str,
    mode: &str,
) {
    use super::provider_i18n;

    let i18n_dir = provider_i18n::resolve_i18n_dir(bundle_root);
    let empty = std::collections::BTreeMap::new();
    let i18n = provider_i18n::load_and_merge(&empty, locale, i18n_dir.as_deref());

    if i18n.is_empty() {
        return;
    }

    // Derive provider prefix: "messaging-telegram" → "telegram"
    let prefix = provider_id
        .strip_prefix("messaging-")
        .or_else(|| provider_id.strip_prefix("events-"))
        .unwrap_or(provider_id);

    // Translate form title
    let title_key = format!("{prefix}.qa.{mode}.title");
    if let Some(title) = i18n.get(&title_key) {
        form_spec.title = title.clone();
    }

    // Translate each question's title and description
    for q in &mut form_spec.questions {
        let q_title_key = format!("{prefix}.qa.{mode}.{}", q.id);
        if let Some(title) = i18n.get(&q_title_key) {
            q.title = title.clone();
        }

        let desc_key = format!("{prefix}.schema.config.{}.description", q.id);
        if let Some(desc) = i18n.get(&desc_key) {
            q.description = Some(desc.clone());
        }
    }

    operator_log::info(
        module_path!(),
        format!(
            "[onboard] i18n fallback: provider={} locale={} keys={}",
            provider_id, locale, i18n.len()
        ),
    );
}

fn find_provider_pack(
    bundle_root: &std::path::Path,
    domain: Domain,
    provider_id: &str,
) -> Result<ProviderPack, Response<Full<Bytes>>> {
    let packs = domains::discover_provider_packs(bundle_root, domain).map_err(|err| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("discover packs: {err}"),
        )
    })?;

    packs
        .into_iter()
        .find(|pack| {
            pack.pack_id == provider_id
                || pack
                    .file_name
                    .strip_suffix(".gtpack")
                    .unwrap_or(&pack.file_name)
                    == provider_id
        })
        .ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                format!("provider pack not found: {provider_id}"),
            )
        })
}

fn parse_domain(body: &Value) -> Result<Domain, Response<Full<Bytes>>> {
    let domain_str = body["domain"].as_str().unwrap_or("messaging");
    match domain_str {
        "messaging" => Ok(Domain::Messaging),
        "events" => Ok(Domain::Events),
        "secrets" => Ok(Domain::Secrets),
        _ => Err(error_response(
            StatusCode::BAD_REQUEST,
            format!("unknown domain: {domain_str}"),
        )),
    }
}

fn parse_mode(body: &Value) -> QaMode {
    match body["mode"].as_str().unwrap_or("setup") {
        "upgrade" => QaMode::Upgrade,
        "remove" => QaMode::Remove,
        "default" => QaMode::Default,
        _ => QaMode::Setup,
    }
}

/// Read existing provider config and merge into answers for upgrade mode.
///
/// Existing config values act as defaults — user-provided answers take priority.
fn merge_existing_config(
    bundle_root: &std::path::Path,
    provider_id: &str,
    answers: &Value,
) -> Value {
    let providers_root = bundle_root.join(".providers");
    let existing = match crate::provider_config_envelope::read_provider_config_envelope(
        &providers_root,
        provider_id,
    ) {
        Ok(Some(envelope)) => envelope.config,
        _ => return answers.clone(),
    };

    let Some(existing_map) = existing.as_object() else {
        return answers.clone();
    };

    let answers_map = answers.as_object().cloned().unwrap_or_default();

    // Start from existing config, then overlay user answers
    let mut merged = existing_map.clone();
    for (key, value) in &answers_map {
        // User-provided answers take priority (even empty strings — user may want to clear)
        merged.insert(key.clone(), value.clone());
    }

    Value::Object(merged)
}

/// Read the public base URL from the runtime state directory.
///
/// The tunnel URL is shared across all tenants — one ngrok/cloudflared
/// endpoint serves the whole operator. We scan all
/// `{state_dir}/runtime/*/public_base_url.txt` and pick the most
/// recently modified file so we always return the current tunnel URL.
fn read_runtime_public_url(
    bundle_root: &std::path::Path,
    _tenant: &str,
    _team: Option<&str>,
) -> Option<String> {
    let runtime_dir = bundle_root.join("state").join("runtime");
    let entries = std::fs::read_dir(&runtime_dir).ok()?;

    let mut best: Option<(std::time::SystemTime, String)> = None;

    for entry in entries.flatten() {
        let url_path = entry.path().join("public_base_url.txt");
        let Ok(meta) = std::fs::metadata(&url_path) else { continue };
        let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let Ok(contents) = std::fs::read_to_string(&url_path) else { continue };
        let trimmed = contents.trim();
        let url = if trimmed.starts_with("https://") {
            trimmed.to_string()
        } else if let Some(parsed) = crate::ngrok::parse_public_url(&contents) {
            parsed
        } else {
            continue;
        };

        if best.as_ref().map_or(true, |(t, _)| modified > *t) {
            best = Some((modified, url));
        }
    }

    best.map(|(_, url)| url)
}

/// After submit, register webhooks with external APIs where applicable.
///
/// This makes native HTTP calls from the operator (not through WASM) so
/// it can reliably reach external APIs. Currently supports Telegram.
fn try_provider_setup_webhook(
    _bundle_root: &std::path::Path,
    _domain: Domain,
    _pack: &ProviderPack,
    provider_id: &str,
    tenant: &str,
    _team: Option<&str>,
    config: &Value,
) -> Option<Value> {
    let public_base_url = config.get("public_base_url").and_then(Value::as_str)?;
    if public_base_url.is_empty() || !public_base_url.starts_with("https://") {
        return None;
    }

    let team = _team.unwrap_or("default");

    // Dispatch based on provider type
    let provider_short = provider_id
        .strip_prefix("messaging-")
        .unwrap_or(provider_id);

    match provider_short {
        "telegram" => setup_telegram_webhook(config, public_base_url, provider_id, tenant, team),
        _ => None,
    }
}

/// Call Telegram Bot API `setWebhook` to register the webhook URL.
fn setup_telegram_webhook(config: &Value, public_base_url: &str, provider_id: &str, tenant: &str, team: &str) -> Option<Value> {
    let bot_token = config.get("bot_token").and_then(Value::as_str)?;
    if bot_token.is_empty() {
        return Some(json!({"ok": false, "error": "bot_token is empty"}));
    }

    let api_base = config
        .get("api_base_url")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.contains("telegram.org"))
        .unwrap_or("https://api.telegram.org");

    let webhook_url = format!(
        "{}/v1/messaging/ingress/{}/{}/{}",
        public_base_url.trim_end_matches('/'),
        provider_id,
        tenant,
        team,
    );

    let url = format!("{api_base}/bot{bot_token}/setWebhook");
    let body = json!({
        "url": webhook_url,
        "allowed_updates": ["message", "callback_query", "edited_message"]
    });

    let token_preview = if bot_token.len() > 10 {
        format!("{}...{}", &bot_token[..5], &bot_token[bot_token.len()-4..])
    } else {
        "***".to_string()
    };
    operator_log::info(
        module_path!(),
        format!("[onboard] telegram setWebhook url={} token_preview={} api={}", webhook_url, token_preview, api_base),
    );

    match ureq::post(&url)
        .header("Content-Type", "application/json")
        .send_json(&body)
    {
        Ok(mut resp) => {
            let status = resp.status().as_u16();
            let raw_body = resp.body_mut().read_to_string().unwrap_or_default();
            operator_log::info(
                module_path!(),
                format!("[onboard] telegram setWebhook response status={} body={}", status, raw_body),
            );
            let resp_body: Value = serde_json::from_str(&raw_body).unwrap_or(Value::Null);
            let tg_ok = resp_body.get("ok").and_then(Value::as_bool).unwrap_or(false);
            let description = resp_body
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            Some(json!({
                "ok": tg_ok,
                "webhook_url": webhook_url,
                "description": description,
                "http_status": status,
                "telegram_response": resp_body,
            }))
        }
        Err(err) => Some(json!({
            "ok": false,
            "error": format!("request failed: {err}"),
            "webhook_url": webhook_url,
        })),
    }
}

/// Build the input payload for a setup flow, mirroring `providers::build_input()`.
fn build_setup_flow_input(
    pack_id: &str,
    tenant: &str,
    team: Option<&str>,
    public_base_url: Option<&str>,
    config: &Value,
) -> Value {
    let team_str = team.unwrap_or("_");
    let mut payload = json!({
        "id": pack_id,
        "tenant": tenant,
        "team": team_str,
        "env": "dev",
    });
    let mut cfg = config.clone();
    if let Some(url) = public_base_url {
        payload["public_base_url"] = Value::String(url.to_string());
        if let Some(map) = cfg.as_object_mut() {
            map.entry("public_base_url".to_string())
                .or_insert_with(|| Value::String(url.to_string()));
        }
    }
    if let Some(map) = cfg.as_object_mut() {
        map.entry("id".to_string())
            .or_insert_with(|| Value::String(pack_id.to_string()));
    }
    payload["config"] = cfg;
    payload["msg"] = json!({
        "channel": "setup",
        "id": format!("{pack_id}.setup"),
        "message": {
            "id": format!("{pack_id}.setup_default__collect"),
            "text": "Collect inputs for setup_default."
        },
        "metadata": {},
        "reply_scope": "",
        "session_id": "setup",
        "tenant_id": tenant,
        "text": "Collect inputs for setup_default.",
        "user_id": "operator"
    });
    payload["payload"] = json!({
        "id": format!("{pack_id}-setup_default"),
        "spec_ref": "assets/setup.yaml"
    });
    payload["setup_answers"] = config.clone();
    if let Ok(answers_str) = serde_json::to_string(config) {
        payload["answers_json"] = Value::String(answers_str);
    }
    payload
}

fn resolve_gmap_path(
    bundle_root: &std::path::Path,
    tenant: &str,
    team: Option<&str>,
) -> std::path::PathBuf {
    match team {
        Some(team) if team != "_" => bundle_root
            .join("tenants")
            .join(tenant)
            .join("teams")
            .join(team)
            .join("team.gmap"),
        _ => bundle_root
            .join("tenants")
            .join(tenant)
            .join("tenant.gmap"),
    }
}
