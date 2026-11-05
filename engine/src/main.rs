#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use arc_swap::ArcSwap;
use engine::autostart::set_autostart;
use engine::config::{default_config, load, save_default_if_missing};
use engine::dispatch::{Dispatcher, SystemExecutor};
use engine::input_win::{run_raw_input, DeviceFilter};
use engine::paths::{config_path, APP_DIR};
use engine::sim::run_simulation;
use engine::tray::create_tray;
use engine::watch::watch_config;
use std::sync::Arc;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use windows::core::w;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Console::{AllocConsole, AttachConsole, ATTACH_PARENT_PROCESS};
use windows::Win32::System::Threading::CreateMutexW;

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
    // Sans dossier de logs on continue sans fichier (la console reste en simulation).
    let file_log = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("engine.log")
        .build(logs)
        .ok()
        .map(|w| {
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(w)
        });
    tracing_subscriber::registry()
        .with(LevelFilter::INFO)
        .with(file_log)
        .with(simulate.then(tracing_subscriber::fmt::layer))
        .init();
    // En release il n'y a pas de console : sans ce hook une panique disparait sans trace.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        default_hook(info);
    }));

    // Instance unique (pas en simulation). Le handle reste ouvert jusqu'a la fin du processus.
    let _single = (!simulate).then(|| {
        let h = unsafe { CreateMutexW(None, false, w!(r"Local\macropad-engine")) };
        if h.is_ok() && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            tracing::info!("deja lance, arret");
            std::process::exit(0);
        }
        h
    });

    let first_launch = !path.exists();
    if let Err(e) = save_default_if_missing(&path) {
        tracing::error!("config par defaut non ecrite: {e}");
    }
    if first_launch && !simulate {
        if let Err(e) = set_autostart(true) {
            tracing::error!("demarrage auto non active: {e}");
        }
    }
    let config = load(&path).unwrap_or_else(|e| {
        tracing::error!("config invalide, config par defaut en memoire: {e}");
        default_config()
    });
    let config = Arc::new(ArcSwap::from_pointee(config));
    // Garde le watcher en vie jusqu'a la fin du programme.
    let _watcher = watch_config(path.clone(), config.clone())
        .map_err(|e| tracing::error!("surveillance de la config impossible: {e}"))
        .ok();
    let dispatcher = Dispatcher::new(config, Arc::new(SystemExecutor));
    if simulate {
        run_simulation(dispatcher);
    } else {
        // Cree ici : la boucle de messages de run_raw_input pompe aussi ceux de l'icone.
        let _tray = create_tray().map_err(|e| tracing::error!("icone non creee: {e}"));
        run_raw_input(dispatcher, DeviceFilter::default());
    }
}
