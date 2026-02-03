pub const REPL_HELP: &str = r#"Available commands:
  @show              ─ display the last adaptive card summary
  @json              ─ emit the raw JSON value received from the flow
  @back              ─ revert to the previous blocked card/inputs
  @input <k>=<v>     ─ set or override an input field
  @click <action_id> ─ submit the card with the provided action
  @help              ─ print this help text
  @quit              ─ exit the REPL"#;

pub fn print_help() {
    println!("{REPL_HELP}");
}
