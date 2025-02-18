use std::process;
use utils::map::HashSet;
use xshell::{cmd, Shell};

use crate::flags::XtaskCmd;

mod flags;

type JsonValue = serde_json::Value;

fn main() -> anyhow::Result<()> {
    let flags = flags::Xtask::from_env()?;
    let sh = Shell::new()?;
    match flags.subcommand {
        XtaskCmd::Dev(_) => {
            cmd!(sh, "cargo lrun -p ehce --features bevy/dynamic_linking").run()?;
        }
        XtaskCmd::Watch(_) => {
            let check = "lclippy";
            cmd!(sh, "cargo watch -x {check}").run()?;
        }
        XtaskCmd::Fix(_) => {
            cmd!(sh, "cargo lfix --allow-dirty --allow-staged -q").run()?;
            cmd!(sh, "cargo lclippy --fix --allow-dirty --allow-staged").run()?;
            cmd!(sh, "cargo fmt --all").run()?;
            cmd!(sh, "cargo sort -w").run()?;
            cmd!(sh, "cargo-machete --fix --skip-target-dir").run()?;
            check_workspace_deps()?;
        }
        XtaskCmd::UnusedDeps(_) => {
            check_workspace_deps()?;
        }
    }

    Ok(())
}

fn check_workspace_deps() -> anyhow::Result<()> {
    fn clear_deps(obj: &JsonValue, dependencies: &mut HashSet<&str>) {
        for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(keys) = obj[key].as_object().map(|o| o.keys()) {
                for k in keys {
                    dependencies.remove(k.as_str());
                }
            }
        }
    }

    let workspace_str = fs_err::read_to_string("Cargo.toml")?;
    let workspace: JsonValue = toml::de::from_str(&workspace_str)?;

    let members = &workspace["workspace"]["members"];
    let mut dependencies = workspace["workspace"]["dependencies"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<HashSet<_>>();

    for member in members.as_array().unwrap() {
        let member_str =
            fs_err::read_to_string(format!("{}/Cargo.toml", member.as_str().unwrap()))?;

        let member: JsonValue = toml::de::from_str(&member_str)?;
        clear_deps(&member, &mut dependencies);

        if let Some(target) = &member["target"].as_object() {
            for value in target.values() {
                clear_deps(value, &mut dependencies);
            }
        }
    }

    if !dependencies.is_empty() {
        println!("\u{001b}[31mUnused workspace dependencies:");
        for dep in dependencies {
            println!("- {}", dep);
        }
        println!("\u{001b}[0m");
        process::exit(1);
    } else {
        println!("\u{001b}[32mNo unused workspace dependencies found\u{001b}[0m");
    }

    Ok(())
}
