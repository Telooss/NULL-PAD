#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use arc_swap::ArcSwap;
use engine::config::{default_config, load, save_default_if_missing};
use engine::dispatch::{Dispatcher, SystemExecutor};
use engine::input_win::{run_raw_input, DeviceFilter};
use engine::paths::{config_path, APP_DIR};
use engine::sim::run_simulation;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use windows::Win32::System::Console::{AllocConsole, AttachConsole, ATTACH_PARENT_PROCESS};

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut simulate, mut path) = (false, config_path());
    while let Some(a) = args.next() {
        match a.as_str() {
            "--simulate" => simulate = true,
            "--config" => path = args.next().map(Into::into).unwrap_or(path),
            other => eprintln!("argument ignore: {other}"),
        }
    }
    if simulate {
        // Sous-systeme "windows" en release : pas de console par defaut.
        unsafe {
            if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
                let _ = AllocConsole();
            }
        }
    }
    let logs = dirs::config_dir()
        .unwrap_or_default()
        .join(APP_DIR)
        .join("logs");
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(tracing_appender::rolling::daily(logs, "engine.log")),
        )
        .with(simulate.then(tracing_subscriber::fmt::layer))
        .init();

    if let Err(e) = save_default_if_missing(&path) {
        tracing::error!("config par defaut non ecrite: {e}");
    }
    let config = load(&path).unwrap_or_else(|e| {
        tracing::error!("config invalide, config par defaut en memoire: {e}");
        default_config()
    });
    let dispatcher = Dispatcher::new(
        Arc::new(ArcSwap::from_pointee(config)),
        Arc::new(SystemExecutor),
    );
    if simulate {
        run_simulation(dispatcher);
    } else {
        run_raw_input(dispatcher, DeviceFilter::default());
    }
}
