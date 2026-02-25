use std::collections::BTreeSet;
use std::path::PathBuf;

fn locale_codes() -> BTreeSet<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("i18n")
        .join("operator_wizard");
    std::fs::read_dir(root)
        .expect("read i18n/operator_wizard")
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".json"))
        .map(|name| name.trim_end_matches(".json").to_string())
        .collect()
}

#[test]
fn includes_requested_locale_sets() {
    let codes = locale_codes();

    let european = [
        "en", "de", "fr", "es", "it", "pt", "ru", "pl", "uk", "ro", "nl", "sv", "cs", "el", "hu",
        "bg", "sr", "hr", "sk", "da", "fi", "no", "lt", "lv", "et",
    ];
    let asian = [
        "zh", "hi", "bn", "ja", "ko", "vi", "id", "ms", "th", "tl", "ur", "fa", "tr", "ta", "te",
        "mr", "gu", "kn", "ml", "pa", "ne", "si", "km", "lo", "my",
    ];
    let arabic_locales = [
        "ar", "ar-EG", "ar-SA", "ar-DZ", "ar-MA", "ar-IQ", "ar-SD", "ar-SY", "ar-TN", "ar-AE",
    ];
    let american = ["qu", "gn", "ay", "nah", "ht"];

    assert!(
        european.iter().all(|code| codes.contains(*code)),
        "missing european locale(s)"
    );
    assert!(
        asian.iter().all(|code| codes.contains(*code)),
        "missing asian locale(s)"
    );
    assert!(
        arabic_locales.iter().all(|code| codes.contains(*code)),
        "missing arabic locale(s)"
    );
    assert!(
        american.iter().all(|code| codes.contains(*code)),
        "missing american locale(s)"
    );
}
