//! Source Raw Input : fenetre message-only qui recoit les rapports HID Consumer du macropad.
use crate::dispatch::Dispatcher;
use crate::report::{parse_consumer_report, ReportTracker};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, RegisterRawInputDevices, HRAWINPUT, RAWINPUTDEVICE,
    RAWINPUTHEADER, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDI_DEVICEINFO, RID_DEVICE_INFO, RID_INPUT,
    RIM_TYPEHID,
};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceFilter {
    pub vid: u16,
    pub pid: u16,
}

impl Default for DeviceFilter {
    /// Defaut ZMK (a confirmer au plan firmware).
    fn default() -> Self {
        Self {
            vid: 0x1D50,
            pid: 0x615E,
        }
    }
}

/// Decoupe le tampon bRawData d'un RAWHID en `count` rapports de `size` octets.
pub fn split_reports(size: u32, count: u32, data: &[u8]) -> impl Iterator<Item = &[u8]> {
    data.chunks_exact(size.max(1) as usize).take(count as usize)
}

struct State {
    dispatcher: Dispatcher,
    tracker: ReportTracker,
    filter: DeviceFilter,
    /// Cache handle de peripherique -> correspond au filtre.
    devices: HashMap<isize, bool>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn matches(filter: DeviceFilter, dev: HANDLE) -> bool {
    let mut info = RID_DEVICE_INFO {
        cbSize: size_of::<RID_DEVICE_INFO>() as u32,
        ..Default::default()
    };
    let mut size = info.cbSize;
    let n = unsafe {
        GetRawInputDeviceInfoW(
            Some(dev),
            RIDI_DEVICEINFO,
            Some(&mut info as *mut _ as *mut _),
            &mut size,
        )
    };
    n != u32::MAX
        && unsafe {
            info.Anonymous.hid.dwVendorId == filter.vid as u32
                && info.Anonymous.hid.dwProductId == filter.pid as u32
        }
}

fn is_ours(st: &mut State, dev: HANDLE) -> bool {
    let filter = st.filter;
    *st.devices
        .entry(dev.0 as isize)
        .or_insert_with(|| matches(filter, dev))
}

fn rearm_timer(hwnd: HWND, st: &State) {
    match st.dispatcher.next_deadline() {
        Some(d) => {
            let ms = d.saturating_duration_since(Instant::now()).as_millis() as u32;
            unsafe { SetTimer(Some(hwnd), 1, ms.max(1), None) };
        }
        None => {
            let _ = unsafe { KillTimer(Some(hwnd), 1) };
        }
    }
}

fn on_wm_input(st: &mut State, lparam: LPARAM) {
    let h = HRAWINPUT(lparam.0 as _);
    let hdr = size_of::<RAWINPUTHEADER>();
    let mut size = 0u32;
    unsafe { GetRawInputData(h, RID_INPUT, None, &mut size, hdr as u32) };
    let mut buf = vec![0u8; size as usize];
    let got = unsafe {
        GetRawInputData(
            h,
            RID_INPUT,
            Some(buf.as_mut_ptr() as *mut _),
            &mut size,
            hdr as u32,
        )
    };
    if got == u32::MAX || buf.len() < hdr + 8 {
        return;
    }
    let header = unsafe { &*(buf.as_ptr() as *const RAWINPUTHEADER) };
    if header.dwType != RIM_TYPEHID.0 || !is_ours(st, header.hDevice) {
        return;
    }
    let u32_at = |o: usize| u32::from_le_bytes(buf[o..o + 4].try_into().unwrap());
    let (hid_size, count) = (u32_at(hdr), u32_at(hdr + 4));
    let now = Instant::now();
    for rep in split_reports(hid_size, count, &buf[hdr + 8..]) {
        for edge in st.tracker.feed(&parse_consumer_report(rep)) {
            st.dispatcher.on_edge(edge, now);
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handled = STATE.with(|s| {
        let mut s = s.borrow_mut();
        let st = s.as_mut()?;
        match msg {
            WM_INPUT => on_wm_input(st, lparam),
            WM_TIMER => st.dispatcher.on_tick(Instant::now()),
            WM_INPUT_DEVICE_CHANGE => {
                let dev = HANDLE(lparam.0 as _);
                if wparam.0 as u32 == GIDC_REMOVAL {
                    if st.devices.remove(&(dev.0 as isize)) == Some(true) {
                        tracing::warn!("macropad removed");
                        // Les Up de clear() sont ignores : device_lost remet les declencheurs a zero.
                        let _ = st.tracker.clear();
                        st.dispatcher.device_lost();
                    }
                } else {
                    is_ours(st, dev);
                }
            }
            _ => return None,
        }
        rearm_timer(hwnd, st);
        Some(())
    });
    match handled {
        Some(()) => LRESULT(0),
        None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub fn run_raw_input(dispatcher: Dispatcher, filter: DeviceFilter) -> ! {
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            dispatcher,
            tracker: ReportTracker::new(),
            filter,
            devices: HashMap::new(),
        })
    });
    let hwnd = create_window().unwrap_or_else(|e| fatal(&format!("window: {e}")));
    let rid = RAWINPUTDEVICE {
        usUsagePage: 0x0C,
        usUsage: 0x01,
        dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
        hwndTarget: hwnd,
    };
    if let Err(e) = unsafe { RegisterRawInputDevices(&[rid], size_of::<RAWINPUTDEVICE>() as u32) } {
        fatal(&format!("RegisterRawInputDevices: {e}"));
    }
    tracing::info!("raw input registered (page 0x0C)");
    let mut msg = MSG::default();
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    std::process::exit(0)
}

fn fatal(m: &str) -> ! {
    tracing::error!("{m}");
    std::process::exit(1)
}

fn create_window() -> windows::core::Result<HWND> {
    unsafe {
        let hinst = GetModuleHandleW(None)?.into();
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst,
            lpszClassName: w!("MacropadEngine"),
            ..Default::default()
        };
        RegisterClassW(&class);
        CreateWindowExW(
            Default::default(),
            class.lpszClassName,
            w!("macropad engine"),
            Default::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinst),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_reports() {
        let data = [1, 2, 3, 4, 5, 6, 7];
        let v: Vec<_> = split_reports(3, 2, &data).collect();
        assert_eq!(v, [&[1, 2, 3][..], &[4, 5, 6][..]]);
        assert_eq!(split_reports(3, 5, &data).count(), 2);
    }
}
