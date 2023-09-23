use crate::states::init_state::InitState;
use crate::states::title_screen_state::TitleScreenState;
use crate::states::{DbeFileSystem, DbeFileSystemBuilder, DbeStateHolder};
use crate::{info_window, DbeState};
use anyhow::Context;
use camino::Utf8PathBuf;
use egui::Ui;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::{error, trace, warn};
use utils::reporter::{report_pair, ReportReceiver, ReportSender, Reporter};

#[derive(Debug)]
pub struct FilesLoadingState {
    path: PathBuf,
    loading: Option<LoadingData>,
}

impl FilesLoadingState {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            loading: None,
        }
    }
}

#[derive(Debug)]
struct LoadingData {
    handle: JoinHandle<()>,
    reporter: ReportReceiver<LoadingProgress, anyhow::Result<DbeFileSystem>>,
}

#[derive(Debug)]
enum FileLoadingProgress {
    Done,
    Skipped,
    Canceled,
}

#[derive(Debug)]
enum LoadingProgress {
    LoadingDirectory(PathBuf),
    LoadingFile(PathBuf),
}

impl Display for LoadingProgress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadingProgress::LoadingDirectory(path) => {
                write!(f, "Loading directory {}", path.to_string_lossy())
            }
            LoadingProgress::LoadingFile(path) => {
                write!(f, "Loading file {}", path.to_string_lossy())
            }
        }
    }
}

fn load_path<T>(
    path: impl AsRef<Path>,
    fs: &mut DbeFileSystemBuilder,
    progress: &ReportSender<LoadingProgress, T>,
) -> anyhow::Result<FileLoadingProgress> {
    if progress.canceled() {
        return Ok(FileLoadingProgress::Canceled);
    }
    let path = path.as_ref();
    if path
        .file_name()
        .context("File has no name???")?
        .to_str()
        .context("Non-UTF filename")?
        .starts_with('.')
    {
        return Ok(FileLoadingProgress::Skipped);
    }
    if path.is_symlink() {
        warn!(
            "Path \"{}\" is a symlink. The editor does not follow symlinks for security reasons",
            path.to_string_lossy()
        );
        return Ok(FileLoadingProgress::Skipped);
    }
    if path.is_dir() {
        progress.progress(LoadingProgress::LoadingDirectory(path.to_path_buf()))?;
        let mut paths = vec![];
        for entry in path.read_dir()? {
            let entry = entry?;
            paths.push(entry.path());
        }

        for p in paths {
            match load_path(&p, fs, progress)
                .with_context(|| format!("While loading path {}", p.to_string_lossy()))?
            {
                FileLoadingProgress::Done => {
                    trace!("Path at {} finished loading", p.to_string_lossy())
                }
                FileLoadingProgress::Skipped => {
                    trace!("Skipped loading file at {}", p.to_string_lossy())
                }
                state @ FileLoadingProgress::Canceled => return Ok(state),
            }
        }
    } else {
        progress.progress(LoadingProgress::LoadingFile(path.to_path_buf()))?;
        let Some(ext) = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
        else {
            return Ok(FileLoadingProgress::Skipped);
        };
        let utf_path: Utf8PathBuf = path
            .to_path_buf()
            .try_into()
            .context("Non-Utf8 paths are not supported")?;
        match ext.as_str() {
            "jpg" | "jpeg" | "png" | "json" | "toml" | "thing" => {
                let data = std::fs::read(path)?;
                fs.raw_files.insert(utf_path, data);
            }
            _ => return Ok(FileLoadingProgress::Skipped),
        }
    }
    Ok(FileLoadingProgress::Done)
}

fn load_files<T>(
    path: PathBuf,
    channel: &ReportSender<LoadingProgress, T>,
) -> anyhow::Result<DbeFileSystem> {
    let mut fs = path.clone().try_into().map(DbeFileSystemBuilder::new)?;
    load_path(path, &mut fs, channel)?;
    fs.build()
}

fn spawn(
    path: impl AsRef<Path>,
    channel: ReportSender<LoadingProgress, anyhow::Result<DbeFileSystem>>,
) -> JoinHandle<()> {
    let path = path.as_ref().to_path_buf();
    std::thread::spawn(move || {
        let files = load_files(path, &channel);
        if channel.canceled() {
            return;
        }
        channel
            .done(files)
            .unwrap_or_else(|_| error!("Main thread has died while loading items"));
    })
}

impl DbeStateHolder for FilesLoadingState {
    fn update(self, ui: &mut Ui) -> DbeState {
        let FilesLoadingState { loading, path } = self;
        let mut loading = match loading {
            None => {
                let (sender, receiver) = report_pair(Reporter::new(
                    LoadingProgress::LoadingDirectory(path.clone()),
                    Duration::from_millis(100),
                ));
                let handle = spawn(&path, sender);
                LoadingData {
                    handle,
                    reporter: receiver,
                }
            }
            Some(loading) => loading,
        };

        if let Some(progress) = loading.reporter.done() {
            return match progress {
                Ok(fs) => {
                    loading
                        .handle
                        .join()
                        .expect("Expect loading thread to terminate successfully");
                    InitState::new(fs).into()
                }
                Err(err) => err.into(),
            };
        }

        info_window(ui, "Loading", |ui| {
            ui.label(loading.reporter.progress().to_string());
            ui.vertical_centered_justified(|ui| {
                if ui.button("Cancel").clicked() {
                    loading.reporter.cancel();
                }
            });
        });

        ui.ctx().request_repaint_after(Duration::from_millis(100));

        if loading.reporter.canceled() {
            return TitleScreenState::new().into();
        }

        FilesLoadingState {
            loading: Some(loading),
            path,
        }
        .into()
    }
}

impl From<FilesLoadingState> for DbeState {
    fn from(value: FilesLoadingState) -> Self {
        DbeState::Loading(value)
    }
}
