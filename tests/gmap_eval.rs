use greentic_operator::gmap::{Policy, eval_policy, eval_with_overlay, parse_path, parse_str};

#[test]
fn eval_precedence_and_overlay() {
    let tenant_rules = parse_str(
        r#"
_ = forbidden
demo-pack/_ = forbidden
demo-pack/main = public
"#,
    )
    .unwrap();
    let team_rules = parse_str(
        r#"
demo-pack/main = forbidden
"#,
    )
    .unwrap();

    let target = parse_path("demo-pack/main", 1).unwrap();
    let tenant_decision = eval_policy(&tenant_rules, &target).unwrap();
    assert_eq!(tenant_decision.policy, Policy::Public);

    let overlay = eval_with_overlay(&tenant_rules, &team_rules, &target).unwrap();
    assert_eq!(overlay.policy, Policy::Forbidden);
}
