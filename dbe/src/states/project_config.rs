use camino::Utf8PathBuf;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectConfig {
    /// Types-related configuration
    pub types: ProjectTypesConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectTypesConfig {
    /// Types directory root
    pub root: Utf8PathBuf,
}

impl Default for ProjectTypesConfig {
    fn default() -> Self {
        Self {
            root: "types".into(),
        }
    }
}
