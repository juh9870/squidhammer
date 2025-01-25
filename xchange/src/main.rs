use clap::Parser;
use std::ffi::OsString;
use tinychange::TinyChangeArgs;

fn main() -> miette::Result<()> {
    tinychange::run(
        TinyChangeArgs::parse_from(
            [OsString::from("cargo xtask")]
                .into_iter()
                .chain(std::env::args_os().skip(2)),
        ),
        "cargo xtask change",
    )
}
