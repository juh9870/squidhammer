use clap::Parser;
use std::ffi::OsString;
use tinychange::TinyChangeArgs;

fn main() -> miette::Result<()> {
    tinychange::run(
        TinyChangeArgs::parse_from(
            [OsString::from("cargo xchange")]
                .into_iter()
                .chain(std::env::args_os().skip(1)),
        ),
        "cargo xchange",
    )
}
