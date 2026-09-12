#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod logging;
mod platform;
mod tray_ui;
mod worker;

use mchose_tray::{battery, icons, settings};

fn main() {
    if let Err(error) = app::run() {
        app::report_error(&error);
        std::process::exit(1);
    }
}
