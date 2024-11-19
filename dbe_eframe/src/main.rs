#![windows_subsystem = "windows"]

use color_backtrace::{default_output_stream, BacktracePrinter};
use dbe_ui::DbeApp;
use eframe::egui::Context;
use eframe::{egui, App, CreationContext, Frame, Storage};
use egui_tracing::tracing::collector::AllowedTargets;
use egui_tracing::EventCollector;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

fn main() -> eframe::Result<()> {
    #[cfg(all(
        debug_assertions,
        target_os = "linux",
        feature = "backtrace-on-stack-overflow"
    ))]
    unsafe {
        backtrace_on_stack_overflow::enable();
    }

    BacktracePrinter::new()
        .add_frame_filter(Box::new(|frame| {
            frame.retain(|frame| {
                if frame
                    .name
                    .as_ref()
                    .is_some_and(|name| name.starts_with("core::ops::function::FnOnce::call_once"))
                {
                    return false;
                }
                true
            })
        }))
        .install(default_output_stream());

    let collector = EventCollector::default()
        .allowed_targets(AllowedTargets::Selected(vec!["dbe".to_string()]));

    let subscriber = tracing_subscriber::Registry::default()
        .with(collector.clone())
        .with(tracing_subscriber::fmt::Layer::default().pretty())
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

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
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
                app.load_storage(&value);
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
