use anyhow::{anyhow, Context};
use camino::Utf8PathBuf;
use clap::Parser;
use dbe::{DbeArguments, DbeState};
use eframe::egui;
use std::env;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Project directory to load
    #[arg(short, long, env = "DBE_PROJECT")]
    pub project: Option<String>,
}
fn main() -> Result<(), anyhow::Error> {
    #[cfg(feature = "debug")]
    unsafe {
        backtrace_on_stack_overflow::enable()
    };
    color_backtrace::install();
    let args = Args::parse();
    tracing_subscriber::fmt::init();
    let project_path = args.project.map(Utf8PathBuf::from);

    let project_dir = match project_path {
        None => None,
        Some(path) => {
            if path.is_absolute() {
                Some(path)
            } else {
                Some(
                    Utf8PathBuf::try_from(
                        env::current_dir()
                            .context("Failed to access current working directory")?
                            .join(path)
                            .canonicalize()?,
                    )
                    .context("Specified directory is not a UTF8 path")?,
                )
            }
        }
    };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Database Editor",
        native_options,
        Box::new(|cc| Box::new(DbeApp::new(cc, project_dir))),
    )
    .map_err(|err| anyhow!("{err}"))?;
    Ok(())
}

#[derive(Debug)]
struct DbeApp {
    data: DbeState,
}

impl DbeApp {
    fn new(_cc: &eframe::CreationContext<'_>, project_dir: Option<Utf8PathBuf>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            data: DbeState::new(DbeArguments {
                project: project_dir,
            }),
        }
    }
}

impl eframe::App for DbeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        dbe::update_dbe(ctx, &mut self.data);
    }
}
