use std::fs;

use greentic_operator::gmap::{Policy, upsert_policy};

#[test]
fn edit_without_comments_is_canonical() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tenant.gmap");
    fs::write(&path, "demo/_ = forbidden\n_ = forbidden\n").unwrap();

    upsert_policy(&path, "demo/main", Policy::Public).unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    let expected = "_ = forbidden\ndemo/_ = forbidden\ndemo/main = public\n";
    assert_eq!(contents, expected);
}

#[test]
fn edit_with_comments_preserves_order() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("team.gmap");
    fs::write(&path, "# team rules\n_ = forbidden\n").unwrap();

    upsert_policy(&path, "demo", Policy::Public).unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    let expected = "# team rules\n_ = forbidden\n\ndemo = public\n";
    assert_eq!(contents, expected);
}
