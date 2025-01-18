#![windows_subsystem = "windows"]

use color_backtrace::{default_output_stream, BacktracePrinter};
use dbe_ui::DbeApp;
use eframe::egui::Context;
use eframe::icon_data::from_png_bytes;
use eframe::{egui, App, CreationContext, Frame, Storage};
use egui_tracing::tracing::collector::AllowedTargets;
use egui_tracing::EventCollector;
use std::fs::File;
use std::io::Write;
use std::{fs, panic};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

const ICON: &[u8] = include_bytes!("../../assets/favicon.png");

fn main() -> eframe::Result<()> {
    #[cfg(all(
        debug_assertions,
        target_os = "linux",
        feature = "backtrace-on-stack-overflow"
    ))]
    unsafe {
        backtrace_on_stack_overflow::enable();
    }

    if fs::exists("dbe.log").expect("Failed to check if log file exists") {
        fs::rename("dbe.log", "dbe.previous.log").expect("Failed to rename log file");
    }

    let log = File::create("dbe.log").expect("Failed to create log file");

    let handler = BacktracePrinter::new().add_frame_filter(Box::new(|frame| {
        frame.retain(|frame| {
            if frame.name.as_ref().is_some_and(|name| {
                name.starts_with("core::ops::function::FnOnce::call_once")
                    || name.starts_with("core::panicking::panic_display")
                    || name.starts_with("core::option::expect_failed")
                    || name.starts_with("core::panicking::assert_failed_inner")
                    || name.starts_with("core::panicking::assert_failed")
            }) {
                return false;
            }
            true
        })
    }));

    panic::set_hook(Box::new(move |info| {
        let err_file = File::create("crash.log").expect("Failed to create crash log file");
        let mut writer = termcolor::NoColor::new(err_file);
        writer.write(
            format!(
                "Something gone extremely wrong.\n\n\
                {} had a problem and crashed.\n\n\
                If you'd like, you can help us diagnose the problem! Please feel free to send us this file.\n\n\
                - Open an issue on GitHub: https://github.com/juh9870/dbe/issues\n\
                - Join support server: https://discord.gg/55dYrPq5Q3\n\
                We take privacy very seriously - we don't perform any automated error collection. In order to improve the software, we rely on users like you to submit reports.\n\n\
                ========================================\n\n\
                Application version: {}\n\
                Operating System: {}\n\
                Architecture: {}\n\n\
                ========================================\n\n\
                ",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::env::consts::ARCH,
            )
            .as_bytes(),
        ).unwrap();
        handler.print_panic_info(info, &mut writer).unwrap();

        let stream = default_output_stream();
        let mut lock = stream.lock();
        handler.print_panic_info(info, &mut lock).unwrap()
    }));

    let collector = EventCollector::default()
        .allowed_targets(AllowedTargets::Selected(vec!["dbe".to_string()]));

    let subscriber = tracing_subscriber::Registry::default()
        .with(collector.clone())
        .with(tracing_subscriber::fmt::Layer::default().pretty())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(log),
        )
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        );

    tracing::subscriber::set_global_default(subscriber).unwrap();

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().min(16))
        .build_global()
        .unwrap();

    let icon = from_png_bytes(ICON).expect("Failed to load icon");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "DBE",
        native_options,
        Box::new(|cx| Ok(Box::new(AppWrapper::new(cx, collector)))),
    )
}

struct AppWrapper(DbeApp);

impl AppWrapper {
    pub fn new(cx: &CreationContext, collector: EventCollector) -> Self {
        DbeApp::register_fonts(&cx.egui_ctx);

        let mut app = DbeApp::new(collector);
        if let Some(storage) = cx.storage {
            if let Some(value) = storage.get_string("dbe") {
                app.load_storage(&cx.egui_ctx, &value);
            }
        }
        Self(app)
    }
}

impl App for AppWrapper {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.0.update(ctx);
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        if let Some(data) = self.0.save_storage() {
            storage.set_string("dbe", data);
        }
    }
}
