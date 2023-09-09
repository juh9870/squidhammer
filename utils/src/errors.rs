use itertools::Itertools;

pub fn display_error(error: impl Into<anyhow::Error>) -> String {
    let error = error.into();
    error.chain().map(|e| e.to_string()).join("\n")
}
