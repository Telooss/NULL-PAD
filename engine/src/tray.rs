use crate::actions::launch;
use crate::autostart::{is_autostart, set_autostart};
use crate::paths::{config_path, APP_DIR};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Icone 32x32 : carre violet #7B5CFF, bordure plus sombre de 2 px.
fn icon() -> Icon {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32u32 {
        for x in 0..32u32 {
            let border = x < 2 || y < 2 || x >= 30 || y >= 30;
            rgba.extend(if border {
                [0x4A, 0x32, 0xB0, 0xFF]
            } else {
                [0x7B, 0x5C, 0xFF, 0xFF]
            });
        }
    }
    Icon::from_rgba(rgba, 32, 32).expect("icone 32x32 valide")
}

/// A creer sur le thread qui pompe les messages Windows ; garder la valeur en vie.
pub fn create_tray() -> Result<TrayIcon, String> {
    let open = MenuItem::new("Ouvrir le dossier de config", true, None);
    let auto = CheckMenuItem::new("Démarrer avec Windows", true, is_autostart(), None);
    let quit = MenuItem::new("Quitter", true, None);
    let menu = Menu::new();
    menu.append_items(&[&open, &auto, &quit])
        .map_err(|e| e.to_string())?;
    let (open_id, auto_id, quit_id) = (open.id().clone(), auto.id().clone(), quit.id().clone());
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        if e.id == quit_id {
            tracing::info!("arret demande depuis l'icone");
            std::process::exit(0);
        } else if e.id == open_id {
            let dir = config_path().parent().map(|p| p.to_path_buf());
            if let Some(dir) = dir {
                if let Err(err) = launch::open(&dir.to_string_lossy()) {
                    tracing::error!("ouverture du dossier impossible: {err}");
                }
            }
        } else if e.id == auto_id {
            // La case se coche seule au clic ; shortcut: pas de resynchro si l'ecriture echoue.
            if let Err(err) = set_autostart(!is_autostart()) {
                tracing::error!("demarrage auto non modifie: {err}");
            }
        }
    }));
    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(APP_DIR)
        .with_icon(icon())
        .build()
        .map_err(|e| e.to_string())
}
