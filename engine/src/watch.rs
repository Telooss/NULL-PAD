use crate::config::{load, Config};
use arc_swap::ArcSwap;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::time::Duration;

const DEBOUNCE: Duration = Duration::from_millis(150);

/// Surveille le dossier (les editeurs remplacent le fichier) ; garder le watcher en vie.
pub fn watch_config(
    path: PathBuf,
    target: Arc<ArcSwap<Config>>,
    on_reload: impl Fn(&Config) + Send + 'static,
) -> notify::Result<RecommendedWatcher> {
    let name = path.file_name().map(|n| n.to_owned());
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
        if let Ok(ev) = ev {
            if ev.paths.iter().any(|p| p.file_name() == name.as_deref()) {
                let _ = tx.send(());
            }
        }
    })?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    std::thread::spawn(move || {
        // Se termine quand le watcher (et donc tx) est lache.
        while rx.recv().is_ok() {
            while rx.recv_timeout(DEBOUNCE).is_ok() {}
            match load(&path) {
                Ok(c) => {
                    // Avant le store : la bibliotheque de sons est chargee avant qu'une liaison Sound neuve ne devienne active.
                    on_reload(&c);
                    target.store(Arc::new(c));
                    tracing::info!("config reloaded");
                }
                Err(e) => tracing::warn!("config rejected: {e}"),
            }
        }
    });
    Ok(watcher)
}
