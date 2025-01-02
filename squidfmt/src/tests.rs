use crate::PreparedFmt;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
// #[test]
// fn test_parse_fine() {
//     insta::assert_debug_snapshot!(PreparedFmt::parse("Hello, {name}!"));
// }

#[test]
fn parsing() {
    insta::glob!("test_cases/parsing/*.txt", |path| {
        let input = fs::read_to_string(path).unwrap();
        insta::assert_debug_snapshot!(PreparedFmt::parse(&input));
    });
}

#[test]
fn formatting() {
    #[derive(Deserialize)]
    struct FmtTestCase {
        format: String,
        values: BTreeMap<String, String>,
    }

    fn snapshot_fmt(path: impl AsRef<Path>) -> String {
        let input = fs::read_to_string(path).unwrap();
        let data: FmtTestCase = toml::from_str(&input).unwrap();

        let parser = PreparedFmt::parse(&data.format).unwrap();
        let result = parser.format_to_string(&data.values).unwrap();

        let report = format!(
            "Format | '{}'\nValues | {}\nResult | '{}'",
            data.format,
            data.values
                .iter()
                .map(|(k, v)| format!("{k}: '{v}'"))
                .collect::<Vec<_>>()
                .join(", "),
            result
        );
        report
    }

    insta::glob!("test_cases/formatting/*.toml", |path| {
        let report = snapshot_fmt(path);
        insta::assert_snapshot!(report);
    });
}
