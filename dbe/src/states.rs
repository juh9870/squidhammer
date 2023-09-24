use std::time::Instant;

use bytesize::ByteSize;
use camino::{Utf8Path, Utf8PathBuf};
use circular_buffer::CircularBuffer;
use egui::{Response, Ui, Visuals};
use pluralizer::pluralize;
use rust_i18n::t;
use rustc_hash::FxHashMap;
use tracing::warn;

use utils::mem_temp;

use crate::states::broken_state::BrokenState;
use crate::states::error_state::ErrorState;
use crate::states::init_state::InitState;
use crate::states::loading_state::FilesLoadingState;
use crate::states::main_state::MainState;
use crate::states::title_screen_state::TitleScreenState;
use crate::vfs::VfsRoot;
use crate::{scale_ui_style, DbeArguments, APP_SCALE_ID};

pub mod broken_state;
pub mod error_state;
pub mod init_state;
pub mod loading_state;
pub mod main_state;
pub mod project_config;
pub mod title_screen_state;

#[enum_dispatch::enum_dispatch]
pub trait DbeStateHolder: Sized {
    fn update(self, ui: &mut Ui) -> DbeState;

    fn layout(mut self, ctx: &egui::Context) -> DbeState {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                scale_ui_style(ui);
                let data = self.update(ui);
                ui.reset_style();
                data
                // ui.ctx().request_repaint();
            })
            .inner
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        default_info_panels(ui);
    }
}

fn light_mode_panel(ui: &mut Ui) {
    if ui.button(t!("dbe.light_mode")).clicked() {
        let visuals = if ui.visuals().dark_mode {
            Visuals::light()
        } else {
            Visuals::dark()
        };
        ui.ctx().set_visuals(visuals);
    }
}
fn ustr_cache_info_panel(ui: &mut Ui) -> Response {
    ui.label(format!(
        "Ustr cache size: {} ({})",
        ByteSize(ustr::total_allocated() as u64),
        pluralize("entry", ustr::num_entries() as isize, true)
    ))
}
fn app_scale_slider(ui: &mut Ui) -> Response {
    ui.horizontal(|ui| {
        ui.label("App scale");
        let mut scale = ui.memory_mut(|mem| *mem.data.get_persisted_mut_or(*APP_SCALE_ID, 1f32));
        ui.add(egui::Slider::new(&mut scale, 0.5..=3.0));
        ui.memory_mut(|mem| mem.data.insert_persisted(*APP_SCALE_ID, scale));
    })
    .response
}

fn bad_fps_panel(ui: &mut Ui) -> Response {
    ui.label("FPS");
    let fps_id = egui::Id::from("FPS");
    let mut buf = mem_temp!(ui, fps_id).unwrap_or_else(CircularBuffer::<128, Instant>::new);
    buf.push_front(Instant::now());
    let elapsed = buf.back().unwrap().elapsed().as_millis() as f64 / 1000.0;
    let fps = buf.len() as f64 / elapsed;
    mem_temp!(ui, fps_id, buf);
    ui.ctx().request_repaint();
    if ui.ctx().frame_nr() % 600 == 0 {
        warn!("FPS counter is active, forcing app to repaint constantly. This message is displayed every on every 600'th frame");
    }
    ui.label(format!("{fps:.2}"))
}

/// Shows default toolbar items
///
/// Layout is inherited from a current UI
fn default_info_panels(ui: &mut Ui) -> Response {
    ui.with_layout(*ui.layout(), |ui| {
        light_mode_panel(ui);
        ui.separator();
        ustr_cache_info_panel(ui);
        ui.separator();
        app_scale_slider(ui);

        // ui.separator();
        // bad_fps_panel(ui);
    })
    .response
}

#[enum_dispatch::enum_dispatch(DbeStateHolder)]
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum DbeState {
    Broken(BrokenState),
    Error(ErrorState),
    TitleScreen(TitleScreenState),
    Loading(FilesLoadingState),
    Initializing(InitState),
    Main(MainState),
}

impl<T: Into<anyhow::Error>> From<T> for DbeState {
    fn from(value: T) -> Self {
        DbeState::Error(ErrorState::new(value.into()))
    }
}

impl DbeState {
    pub fn new(args: DbeArguments) -> Self {
        if let Some(path) = args.project {
            Self::Loading(FilesLoadingState::new(path.into()))
        } else {
            Self::TitleScreen(TitleScreenState::default())
        }
    }
}

#[derive(Debug)]
pub struct DbeFileSystem {
    root: Utf8PathBuf,
    raw_files: FxHashMap<Utf8PathBuf, Vec<u8>>,
    fs: VfsRoot,
}

impl DbeFileSystem {
    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    pub fn fs(&self) -> &VfsRoot {
        &self.fs
    }

    pub fn fs_mut(&mut self) -> &mut VfsRoot {
        &mut self.fs
    }

    pub fn content(&self, path: &Utf8Path) -> Option<&Vec<u8>> {
        self.raw_files.get(path)
    }
}

#[derive(Debug)]
pub struct DbeFileSystemBuilder {
    pub root: Utf8PathBuf,
    pub raw_files: FxHashMap<Utf8PathBuf, Vec<u8>>,
}

impl DbeFileSystemBuilder {
    pub fn new(root: Utf8PathBuf) -> Self {
        Self {
            root,
            raw_files: Default::default(),
        }
    }
    pub fn build(self) -> anyhow::Result<DbeFileSystem> {
        let mut fs = VfsRoot::new(self.root.clone());
        for path in self.raw_files.keys() {
            fs.create(path.clone())?;
        }
        Ok(DbeFileSystem {
            root: self.root,
            raw_files: self.raw_files,
            fs,
        })
    }
}
