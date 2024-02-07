use xshell::{cmd, Shell};

use crate::flags::XtaskCmd;

mod flags;

fn main() -> anyhow::Result<()> {
    let flags = flags::Xtask::from_env()?;
    let sh = Shell::new()?;
    match flags.subcommand {
        XtaskCmd::Dev(_) => {
            cmd!(sh, "cargo lrun -p ehce --features bevy/dynamic_linking").run()?;
        }
        XtaskCmd::Watch(_) => {
            let check = "lcheck";
            cmd!(sh, "cargo watch -x {check}").run()?;
        }
        XtaskCmd::Fix(_) => {
            cmd!(sh, "cargo fmt --all").run()?;
            cmd!(sh, "cargo fix --allow-dirty --allow-staged -q").run()?;
            cmd!(sh, "cargo clippy --fix --allow-dirty --allow-staged").run()?;
            cmd!(sh, "cargo sort -w").run()?;
            cmd!(sh, "cargo-machete --fix --skip-target-dir").run()?;
        }
    }

    Ok(())
}
