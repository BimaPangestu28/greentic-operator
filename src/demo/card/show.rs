use super::CardView;

pub fn print_card_summary(view: &CardView) {
    let summary = view
        .summary_text
        .as_deref()
        .or(view.title.as_deref())
        .unwrap_or("Adaptive card");
    println!("Card received: {summary}");
    if !view.body_texts.is_empty() {
        println!("  body:");
        for text in &view.body_texts {
            println!("    {text}");
        }
    }
    if !view.inputs.is_empty() {
        println!("  inputs:");
        for input in &view.inputs {
            let label = input.label.as_deref().unwrap_or(input.id.as_str());
            let type_desc = input.input_type.as_deref().unwrap_or("input");
            println!("    - {label} (id={}: type={type_desc})", input.id);
            if let Some(placeholder) = input.placeholder.as_deref() {
                println!("      placeholder: {placeholder}");
            }
        }
    }
    if !view.actions.is_empty() {
        println!("  actions:");
        for action in &view.actions {
            let title = action.title.as_deref().unwrap_or(action.id.as_str());
            let kind = action.action_type.as_deref().unwrap_or("action");
            println!("    - {title} (id={}: type={kind})", action.id);
        }
    }
    println!(
        "Hint: @input <field>=<value> to set inputs, @click <action_id> to submit, @show to revisit the card, @json to view raw payload."
    );
}
