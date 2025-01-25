use crate::error::report_error;
use miette::miette;
use std::sync::mpsc::Sender;
use std::time::Duration;
use update_informer::{Check, Version};

/// Check for updates.
///
/// Will spawn a new thread to check for updates, or run the check in the
/// current thread if spawning fails.
pub fn check_for_updates(sender: Sender<Version>, app_version: String) {
    let sender_moved = sender.clone();
    let app_version_moved = app_version.clone();
    if std::thread::Builder::new()
        .spawn(move || check_for_updates_blocking(sender_moved, app_version_moved))
        .is_err()
    {
        check_for_updates_blocking(sender, app_version);
    }
}

fn check_for_updates_blocking(sender: Sender<Version>, app_version: String) {
    let name = "juh9870/squidhammer";
    let version = app_version;
    let informer = update_informer::new(update_informer::registry::GitHub, name, version)
        .interval(Duration::ZERO);

    if let Some(new_version) = informer.check_version().unwrap_or_else(|err| {
        report_error(miette!("{}", err).context("Failed to check for updates"));
        None
    }) {
        if sender.send(new_version).is_err() {
            report_error(miette!("Failed to send new version to main thread"));
        }
    }
}
