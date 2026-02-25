use super::CardView;
use crate::operator_i18n;

pub fn print_card_summary(view: &CardView) {
    let adaptive_card_fallback = operator_i18n::tr("demo.card.adaptive_card", "Adaptive card");
    let summary = view
        .summary_text
        .as_deref()
        .or(view.title.as_deref())
        .unwrap_or(adaptive_card_fallback.as_str());
    println!(
        "{}",
        operator_i18n::trf("demo.card.received", "Card received: {}", &[summary])
    );
    if !view.body_texts.is_empty() {
        println!("{}", operator_i18n::tr("demo.card.body", "  body:"));
        for text in &view.body_texts {
            println!("    {text}");
        }
    }
    if !view.inputs.is_empty() {
        println!("{}", operator_i18n::tr("demo.card.inputs", "  inputs:"));
        for input in &view.inputs {
            let label = input.label.as_deref().unwrap_or(input.id.as_str());
            let input_fallback = operator_i18n::tr("demo.card.input", "input");
            let type_desc = input
                .input_type
                .as_deref()
                .unwrap_or(input_fallback.as_str());
            println!(
                "{}",
                operator_i18n::trf(
                    "demo.card.input_line",
                    "    - {} (id={}: type={})",
                    &[label, &input.id, type_desc]
                )
            );
            if let Some(placeholder) = input.placeholder.as_deref() {
                println!(
                    "{}",
                    operator_i18n::trf(
                        "demo.card.placeholder",
                        "      placeholder: {}",
                        &[placeholder]
                    )
                );
            }
        }
    }
    if !view.actions.is_empty() {
        println!("{}", operator_i18n::tr("demo.card.actions", "  actions:"));
        for action in &view.actions {
            let title = action.title.as_deref().unwrap_or(action.id.as_str());
            let action_fallback = operator_i18n::tr("demo.card.action", "action");
            let kind = action
                .action_type
                .as_deref()
                .unwrap_or(action_fallback.as_str());
            println!(
                "{}",
                operator_i18n::trf(
                    "demo.card.action_line",
                    "    - {} (id={}: type={})",
                    &[title, &action.id, kind]
                )
            );
        }
    }
    println!(
        "{}",
        operator_i18n::tr(
            "demo.card.hint",
            "Hint: @input <field>=<value> to set inputs, @click <action_id> to submit, @show to revisit the card, @json to view raw payload."
        )
    );
}
