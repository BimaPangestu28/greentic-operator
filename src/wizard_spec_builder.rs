use serde_json::{Value, json};

use crate::wizard::{QaQuestion, QaSpec, WizardMode};

pub fn build_spec(mode: WizardMode) -> QaSpec {
    QaSpec {
        mode: mode.as_str().to_string(),
        questions: vec![
            QaQuestion {
                id: "operator.bundle.path".to_string(),
                title: "Bundle output path".to_string(),
                required: true,
            },
            QaQuestion {
                id: "operator.packs.refs".to_string(),
                title: "Pack refs (catalog + custom)".to_string(),
                required: false,
            },
            QaQuestion {
                id: "operator.tenants".to_string(),
                title: "Tenants and optional teams".to_string(),
                required: true,
            },
            QaQuestion {
                id: "operator.allow.paths".to_string(),
                title: "Allow rules as PACK[/FLOW[/NODE]]".to_string(),
                required: false,
            },
        ],
    }
}

pub fn build_validation_form(mode: WizardMode) -> Value {
    build_validation_form_with_providers(mode, &[])
}

pub fn build_validation_form_with_providers(mode: WizardMode, provider_ids: &[String]) -> Value {
    match mode {
        WizardMode::Create => create_validation_form(provider_ids),
        WizardMode::Update => update_validation_form(provider_ids),
        WizardMode::Remove => remove_validation_form(),
    }
}

fn create_validation_form(provider_ids: &[String]) -> Value {
    let provider_field = if provider_ids.is_empty() {
        json!({ "id": "provider_id", "type": "string", "title": "Provider id", "required": true })
    } else {
        json!({
            "id": "provider_id",
            "type": "enum",
            "title": "Provider id",
            "required": true,
            "choices": provider_ids
        })
    };
    json!({
        "id": "operator.wizard.create",
        "title": "Create bundle",
        "version": "1.0.0",
        "presentation": { "default_locale": "en-GB" },
        "questions": [
            {
                "id": "bundle_path",
                "type": "string",
                "title": "Bundle output path",
                "title_i18n": { "key": "wizard.create.bundle_path" },
                "required": true
            },
            {
                "id": "bundle_name",
                "type": "string",
                "title": "Bundle name",
                "title_i18n": { "key": "wizard.create.bundle_name" },
                "required": true
            },
            {
                "id": "locale",
                "type": "string",
                "title": "Locale",
                "title_i18n": { "key": "wizard.create.locale" },
                "required": false
            },
            {
                "id": "pack_refs",
                "type": "list",
                "title": "Pack references",
                "title_i18n": { "key": "wizard.create.pack_refs" },
                "required": false,
                "list": {
                    "fields": [
                        { "id": "pack_ref", "type": "string", "title": "Pack ref", "required": true },
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": false },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false },
                        {
                            "id": "make_default_scope",
                            "type": "enum",
                            "title": "Default scope",
                            "required": false,
                            "choices": ["none", "global", "tenant", "team"]
                        }
                    ]
                }
            },
            {
                "id": "providers",
                "type": "list",
                "title": "Providers",
                "title_i18n": { "key": "wizard.create.providers" },
                "required": false,
                "list": {
                    "fields": [provider_field]
                }
            },
            {
                "id": "targets",
                "type": "list",
                "title": "Tenants and teams",
                "title_i18n": { "key": "wizard.create.targets" },
                "required": true,
                "list": {
                    "fields": [
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": true },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false }
                    ]
                }
            },
            {
                "id": "access_mode",
                "type": "enum",
                "title": "Access mode",
                "title_i18n": { "key": "wizard.create.access_mode" },
                "required": true,
                "choices": ["all_selected_get_all_packs", "per_pack_matrix"]
            },
            {
                "id": "access_change",
                "type": "list",
                "title": "Per-pack access matrix",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "pack_id", "type": "string", "title": "Pack id", "required": true },
                        {
                            "id": "operation",
                            "type": "enum",
                            "title": "Operation",
                            "required": true,
                            "choices": ["allow_add", "allow_remove"]
                        },
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": true },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false }
                    ]
                }
            },
            {
                "id": "execution_mode",
                "type": "enum",
                "title": "Execution mode",
                "title_i18n": { "key": "wizard.create.execution_mode" },
                "required": true,
                "choices": ["dry_run", "execute"]
            }
        ],
        "validations": [
            {
                "id": "team_requires_tenant",
                "message": "team_id requires tenant_id",
                "fields": ["tenant_id", "team_id"],
                "condition": {
                    "op": "and",
                    "expressions": [
                        { "op": "is_set", "path": "team_id" },
                        { "op": "not", "expression": { "op": "is_set", "path": "tenant_id" } }
                    ]
                },
                "code": "team_requires_tenant"
            }
        ]
    })
}

fn update_validation_form(provider_ids: &[String]) -> Value {
    let provider_field = if provider_ids.is_empty() {
        json!({ "id": "provider_id", "type": "string", "title": "Provider id", "required": true })
    } else {
        json!({
            "id": "provider_id",
            "type": "enum",
            "title": "Provider id",
            "required": true,
            "choices": provider_ids
        })
    };
    json!({
        "id": "operator.wizard.update",
        "title": "Update bundle",
        "version": "1.0.0",
        "presentation": { "default_locale": "en-GB" },
        "questions": [
            {
                "id": "bundle_path",
                "type": "string",
                "title": "Bundle path",
                "title_i18n": { "key": "wizard.update.bundle_path" },
                "required": true
            },
            {
                "id": "update_ops",
                "type": "list",
                "title": "Update operations",
                "title_i18n": { "key": "wizard.update.ops" },
                "required": false,
                "list": {
                    "fields": [
                        {
                            "id": "op",
                            "type": "enum",
                            "title": "Operation",
                            "required": true,
                            "choices": [
                                "packs_add",
                                "packs_remove",
                                "providers_add",
                                "providers_remove",
                                "tenants_add",
                                "tenants_remove",
                                "access_change"
                            ]
                        }
                    ]
                }
            },
            {
                "id": "pack_refs",
                "type": "list",
                "title": "Packs to add",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "pack_ref", "type": "string", "title": "Pack ref", "required": true }
                    ]
                }
            },
            {
                "id": "packs_remove",
                "type": "list",
                "title": "Packs to remove",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "pack_identifier", "type": "string", "title": "Pack id/ref", "required": true },
                        {
                            "id": "scope",
                            "type": "enum",
                            "title": "Scope",
                            "required": false,
                            "choices": ["bundle", "global", "tenant", "team"]
                        },
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": false },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false }
                    ]
                }
            },
            {
                "id": "providers",
                "type": "list",
                "title": "Providers to enable",
                "required": false,
                "list": {
                    "fields": [provider_field.clone()]
                }
            },
            {
                "id": "providers_remove",
                "type": "list",
                "title": "Providers to disable",
                "required": false,
                "list": {
                    "fields": [provider_field]
                }
            },
            {
                "id": "targets",
                "type": "list",
                "title": "Tenants and teams to add/update",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": true },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false }
                    ]
                }
            },
            {
                "id": "tenants_remove",
                "type": "list",
                "title": "Tenants/teams to remove",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": true },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false }
                    ]
                }
            },
            {
                "id": "access_change",
                "type": "list",
                "title": "Access changes",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "pack_id", "type": "string", "title": "Pack id", "required": true },
                        {
                            "id": "operation",
                            "type": "enum",
                            "title": "Operation",
                            "required": true,
                            "choices": ["allow_add", "allow_remove"]
                        },
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": true },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false }
                    ]
                }
            },
            {
                "id": "execution_mode",
                "type": "enum",
                "title": "Execution mode",
                "title_i18n": { "key": "wizard.update.execution_mode" },
                "required": true,
                "choices": ["dry_run", "execute"]
            }
        ],
        "validations": []
    })
}

fn remove_validation_form() -> Value {
    json!({
        "id": "operator.wizard.remove",
        "title": "Remove from bundle",
        "version": "1.0.0",
        "presentation": { "default_locale": "en-GB" },
        "questions": [
            {
                "id": "bundle_path",
                "type": "string",
                "title": "Bundle path",
                "title_i18n": { "key": "wizard.remove.bundle_path" },
                "required": true
            },
            {
                "id": "remove_targets",
                "type": "list",
                "title": "Remove targets",
                "title_i18n": { "key": "wizard.remove.targets" },
                "required": false,
                "list": {
                    "fields": [
                        {
                            "id": "target_type",
                            "type": "enum",
                            "title": "Target type",
                            "required": true,
                            "choices": ["packs", "providers", "tenants_teams"]
                        },
                        { "id": "target_id", "type": "string", "title": "Target id", "required": true }
                    ]
                }
            },
            {
                "id": "packs_remove",
                "type": "list",
                "title": "Packs to remove",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "pack_identifier", "type": "string", "title": "Pack id/ref", "required": true },
                        {
                            "id": "scope",
                            "type": "enum",
                            "title": "Scope",
                            "required": false,
                            "choices": ["bundle", "global", "tenant", "team"]
                        },
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": false },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false }
                    ]
                }
            },
            {
                "id": "providers_remove",
                "type": "list",
                "title": "Providers to remove",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "provider_id", "type": "string", "title": "Provider id", "required": true }
                    ]
                }
            },
            {
                "id": "tenants_remove",
                "type": "list",
                "title": "Tenants/teams to remove",
                "required": false,
                "list": {
                    "fields": [
                        { "id": "tenant_id", "type": "string", "title": "Tenant id", "required": true },
                        { "id": "team_id", "type": "string", "title": "Team id", "required": false }
                    ]
                }
            },
            {
                "id": "execution_mode",
                "type": "enum",
                "title": "Execution mode",
                "title_i18n": { "key": "wizard.remove.execution_mode" },
                "required": true,
                "choices": ["dry_run", "execute"]
            }
        ],
        "validations": []
    })
}
