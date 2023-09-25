use anyhow::{anyhow, Context};
use egui::Ui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use tracing::trace;

use utils::errors::display_error;

use crate::dbe_files::DbeFileSystem;
use crate::states::main_state::MainState;
use crate::states::project_config::ProjectConfig;
use crate::states::DbeStateHolder;
use crate::value::etype::registry::ETypesRegistry;
use crate::value::JsonValue;
use crate::{info_window, DbeState};

#[derive(Debug)]
pub enum InitState {
    Init(DbeFileSystem),
    Ready(DbeFileSystem, ETypesRegistry),
    Error(String),
}

impl InitState {
    pub fn new(fs: DbeFileSystem) -> Self {
        Self::Init(fs)
    }
}

fn init_editor(fs: &mut DbeFileSystem) -> anyhow::Result<ETypesRegistry> {
    let mut registry_items = vec![];

    let config = fs
        .content(&fs.root().join("things_editor.toml"))
        .context("`things_editor.toml` is missing. Are you sure this is a valid project folder?")?;

    let mut config: ProjectConfig = toml::de::from_str(
        std::str::from_utf8(
            config
                .as_raw()
                .expect("Config item should be raw at this point"),
        )
        .map_err(|_| {
            anyhow!(
                "Invalid file encoding, please check that `things_editor.toml` is encoded in UTF8"
            )
        })?,
    )
    .context("While parsing `things_editor.toml`")?;

    config.types.root = fs.root().join(config.types.root).canonicalize_utf8()?;

    anyhow::ensure!(
        config.types.root.starts_with(fs.root()),
        "`types_folder` option point to path outside of project root directory"
    );

    for (path, data) in fs.fs().iter() {
        let Some(ext) = path.extension().map(|e| e.to_ascii_lowercase()) else {
            continue;
        };
        let raw_data = data
            .as_raw()
            .expect("All files should be raw at this point");

        match ext.as_ref() {
            "thing" => {
                let value: JsonValue = serde_json5::from_slice(raw_data.as_slice())
                    .with_context(|| format!("While parsing file at \"{path}\""))?;
                registry_items.push((path.clone(), value));

                trace!("Deserialized thing at {path}");
            }
            "json" => {}
            _ => {}
        }
    }

    ETypesRegistry::from_raws(config.types.root.clone(), registry_items).with_context(|| {
        format!(
            "While initializing types registry\nRoot folder: `{}`",
            config.types.root
        )
    })
}

impl DbeStateHolder for InitState {
    fn update(self, ui: &mut Ui) -> DbeState {
        match self {
            InitState::Init(mut fs) => match init_editor(&mut fs)
                .with_context(|| format!("While loading project directory at `{}`", fs.root()))
            {
                Ok(reg) => Self::Ready(fs, reg).into(),
                Err(err) => Self::Error(display_error(err)).into(),
            },
            InitState::Ready(fs, reg) => MainState::new(fs, reg).into(),
            InitState::Error(err) => {
                info_window(ui, "Something gone wrong", |ui| {
                    let mut cache = CommonMarkCache::default();
                    CommonMarkViewer::new("error_viewer").show(ui, &mut cache, &err)
                });
                InitState::Error(err).into()
            }
        }
    }
}
