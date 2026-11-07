#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use arc_swap::ArcSwap;
use engine::autostart::{migrate_autostart, set_autostart};
use engine::config::{default_config, load, save_default_if_missing};
use engine::dispatch::{Dispatcher, SystemExecutor};
use engine::input_win::{run_raw_input, DeviceFilter};
use engine::midi::Midi;
use engine::obs::start_obs;
use engine::paths::{config_path, migrate_app_dir, APP_DIR, OLD_APP_DIR};
use engine::sim::run_simulation;
use engine::sound::board::Soundboard;
use engine::sound::devices::{cable_present, start_audio};
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
    let base = dirs::config_dir().unwrap_or_default();
    // Avant les logs : le dossier NULL-PAD est renomme avant d'y ecrire.
    let migrated = migrate_app_dir(&base);
    let logs = base.join(APP_DIR).join("logs");
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

    if let Err(e) = migrated {
        tracing::error!("migration du dossier NULL-PAD impossible: {e}");
        // Ne pas ecrire une config par defaut par-dessus une migration en attente.
        let old = base.join(OLD_APP_DIR).join("config.json");
        if path == config_path() && old.exists() && !path.exists() {
            tracing::warn!(
                "ancienne config utilisee pour cette session: {}",
                old.display()
            );
            path = old;
        }
    }
    if !simulate {
        migrate_autostart();
    }

    // Instance unique (pas en simulation). Le handle reste ouvert jusqu'a la fin du processus.
    let _single = (!simulate).then(|| {
        let h = unsafe { CreateMutexW(None, false, w!(r"Local\nullctl-engine")) };
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
    let board = Soundboard::new();
    let audio = start_audio(board.clone());
    let config = load(&path).unwrap_or_else(|e| {
        tracing::error!("config invalide, config par defaut en memoire: {e}");
        default_config()
    });
    board.apply(&config.sound);
    audio.apply(&config.sound);
    let obs = start_obs();
    let midi = Midi::new();
    obs.apply(&config.obs);
    midi.apply(&config.midi);
    let config = Arc::new(ArcSwap::from_pointee(config));
    let (b, a, o, m) = (board.clone(), audio.clone(), obs.clone(), midi.clone());
    // Garde le watcher en vie jusqu'a la fin du programme.
    let _watcher = watch_config(path.clone(), config.clone(), move |c| {
        b.apply(&c.sound);
        a.apply(&c.sound);
        o.apply(&c.obs);
        m.apply(&c.midi);
    })
    .map_err(|e| tracing::error!("surveillance de la config impossible: {e}"))
    .ok();
    let dispatcher = Dispatcher::new(
        config,
        Arc::new(SystemExecutor {
            sound: board,
            obs,
            midi,
        }),
    );
    if simulate {
        run_simulation(dispatcher);
    } else {
        // Cree ici : la boucle de messages de run_raw_input pompe aussi ceux de l'icone.
        let _tray =
            create_tray(!cable_present()).map_err(|e| tracing::error!("icone non creee: {e}"));
        run_raw_input(dispatcher, DeviceFilter::default());
    }
}
