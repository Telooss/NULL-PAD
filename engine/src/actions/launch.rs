use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use windows::core::{w, HSTRING};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Starts a program without waiting; its window stays visible.
pub fn launch(program: &str, args: &[String]) -> Result<(), String> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|e| format!("{program}: {e}"))
}

/// Opens a file, folder or URL with its default handler.
pub fn open(target: &str) -> Result<(), String> {
    let r = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(target),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW returns a value > 32 on success.
    if r.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("{target}: ShellExecute code {}", r.0 as usize))
    }
}

/// Runs a PowerShell line, hidden, without waiting.
pub fn command(line: &str) -> Result<(), String> {
    Command::new("powershell.exe")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", line])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(drop)
        .map_err(|e| format!("powershell: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn launch_missing_program_is_err() {
        assert!(launch(r"C:\nope\absent.exe", &[]).is_err());
    }

    #[test]
    fn command_runs() {
        let name = format!("mp_engine_test_{}.txt", std::process::id());
        let path = std::env::temp_dir().join(&name);
        let _ = std::fs::remove_file(&path);
        command(&format!(
            "New-Item -ItemType File -Path $env:TEMP\\{name} -Force"
        ))
        .unwrap();
        let start = Instant::now();
        while !path.exists() && start.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(50));
        }
        let ok = path.exists();
        let _ = std::fs::remove_file(&path);
        assert!(ok, "file not created within 3 s");
    }
}
