use crate::codegen::Ctx;
use camino::Utf8PathBuf;
use clap::Parser;
use codegen_schema::schema::{SchemaDataType, SchemaItem};
use miette::Context;
use std::path::PathBuf;
use tracing_panic::panic_hook;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

mod codegen;

/// Generates typescript definitions for items from Event Horizon schema
#[derive(Debug, Parser)]
struct Args {
    /// Path to the schema directory
    #[arg(short, long, env = "CODEGEN_SCHEMA_INPUT")]
    schema: PathBuf,
    /// Path to the output directory
    #[arg(short, long, env = "CODEGEN_OUTPUT")]
    output: PathBuf,
}

pub fn main() -> miette::Result<()> {
    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::Layer::default().pretty())
        .with(EnvFilter::from_default_env());

    tracing::subscriber::set_global_default(subscriber).unwrap();

    color_backtrace::install();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        prev_hook(panic_info);
    }));

    m_try(|| {
        let Args { schema, output } = Args::parse();

        let mut ctx = Ctx::default();
        let files = codegen_schema::load_from_dir(&schema)?;
        for (path, item) in files {
            let relative = Utf8PathBuf::from_path_buf(
                path.strip_prefix(&schema)
                    .expect("Path should be in schema directory")
                    .to_path_buf(),
            )
            .expect("Path should be valid utf8");
            match item {
                SchemaItem::Schema { .. } => {}
                SchemaItem::Data(data) => match &data.ty {
                    SchemaDataType::Enum => ctx.consume_enum(relative, data),
                    SchemaDataType::Expression => {}
                    SchemaDataType::Struct | SchemaDataType::Settings | SchemaDataType::Object => {
                        ctx.consume_struct(relative, data)
                    }
                },
            }
        }

        ctx.finish(&output);

        Ok(())
    })
    .context("Code generator failed")
}

/// Helper for wrapping a code block to help with contextualizing errors
/// Better editor support but slightly worse ergonomic than a macro
#[inline(always)]
pub(crate) fn m_try<T>(func: impl FnOnce() -> miette::Result<T>) -> miette::Result<T> {
    func()
}
