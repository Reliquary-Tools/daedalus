use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(target_os = "windows")]
use std::os::windows::{ffi::OsStrExt, process::CommandExt};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn launch_allowed(app_slug: &str, app_name: &str) -> bool {
    if cfg!(debug_assertions) {
        return true;
    }

    if launched_from_obelisk_with_license_file() {
        return true;
    }

    match validate_with_obelisk(app_slug) {
        Ok(()) => {
            ensure_obelisk_running(app_slug);
            true
        }
        Err(error) => {
            let _ = launch_obelisk(app_slug);
            show_license_error(app_name, &error);
            false
        }
    }
}

fn launched_from_obelisk_with_license_file() -> bool {
    let launched_from_obelisk = env::var("RELIQUARY_OBELISK_LAUNCH")
        .map(|value| value == "1")
        .unwrap_or(false);
    let license_file_exists = env::var("RELIQUARY_LICENSE_FILE")
        .map(|path| Path::new(&path).is_file())
        .unwrap_or(false);

    launched_from_obelisk && license_file_exists
}

fn validate_with_obelisk(app_slug: &str) -> Result<(), String> {
    let executable = resolve_obelisk_executable(app_slug)
        .ok_or_else(|| "Obelisk is required to validate the RELIQUARY license.".to_string())?;
    let output = silent_command(&executable)
        .args(["--reliquary-check-license", app_slug])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Unable to ask Obelisk for license validation: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "No valid RELIQUARY license is activated for this app.".to_string()
    })
}

fn ensure_obelisk_running(app_slug: &str) {
    if is_process_running("obelisk.exe") {
        return;
    }

    let _ = launch_obelisk(app_slug);
}

fn launch_obelisk(app_slug: &str) -> Result<(), String> {
    let executable = resolve_obelisk_executable(app_slug)
        .ok_or_else(|| "Obelisk is not installed yet.".to_string())?;
    silent_command(executable)
        .spawn()
        .map_err(|error| format!("Unable to open Obelisk: {error}"))?;

    Ok(())
}

fn resolve_obelisk_executable(app_slug: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(root) = reliquary_workspace_root(app_slug) {
        let obelisk_root = root.join("obelisk");
        candidates.push(
            obelisk_root
                .join("src-tauri")
                .join("target")
                .join("release")
                .join("obelisk.exe"),
        );
        candidates.push(
            obelisk_root
                .join("src-tauri")
                .join("target")
                .join("debug")
                .join("obelisk.exe"),
        );
    }

    if let Some(root) = reliquary_install_root(app_slug) {
        candidates.push(root.join("Obelisk").join("obelisk.exe"));
        candidates.push(root.join("obelisk").join("obelisk.exe"));
    }

    for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(base) = env::var(key) {
            let base = PathBuf::from(base);
            candidates.push(base.join("Reliquary").join("Obelisk").join("obelisk.exe"));
            candidates.push(base.join("RELIQUARY").join("Obelisk").join("obelisk.exe"));
            candidates.push(base.join("Obelisk").join("obelisk.exe"));
            candidates.push(base.join("Programs").join("Obelisk").join("obelisk.exe"));
            candidates.push(
                base.join("Programs")
                    .join("Reliquary")
                    .join("Obelisk")
                    .join("obelisk.exe"),
            );
            candidates.push(
                base.join("Programs")
                    .join("RELIQUARY")
                    .join("Obelisk")
                    .join("obelisk.exe"),
            );
        }
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn reliquary_workspace_root(app_slug: &str) -> Option<PathBuf> {
    let mut starts = Vec::new();

    if let Ok(path) = env::current_dir() {
        starts.push(path);
    }

    if let Ok(path) = env::current_exe() {
        if let Some(parent) = path.parent() {
            starts.push(parent.to_path_buf());
        }
    }

    for start in starts {
        for ancestor in start.ancestors() {
            if ancestor.join("obelisk").is_dir() && ancestor.join(app_slug).is_dir() {
                return Some(ancestor.to_path_buf());
            }
        }
    }

    None
}

fn reliquary_install_root(app_slug: &str) -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let app_dir = exe.parent()?;
    let root = app_dir.parent()?;
    let app_name = app_dir.file_name()?.to_string_lossy();

    app_name
        .eq_ignore_ascii_case(app_slug)
        .then(|| root.to_path_buf())
}

#[cfg(target_os = "windows")]
fn is_process_running(exe_name: &str) -> bool {
    let filter = format!("IMAGENAME eq {exe_name}");
    let output = silent_command("tasklist")
        .args(["/FI", &filter, "/FO", "CSV", "/NH"])
        .output();

    output
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains(exe_name))
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_process_running(_exe_name: &str) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn show_license_error(app_name: &str, detail: &str) {
    extern "system" {
        fn MessageBoxW(
            hwnd: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }

    let message = format!(
        "No valid RELIQUARY license is activated for {app_name}.\n\nOpen Obelisk and activate or refresh a Suite license.\n\nDetails: {detail}"
    );
    let caption = format!("{app_name} - RELIQUARY license required");
    let message = wide_null(&message);
    let caption = wide_null(&caption);

    unsafe {
        let _ = MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            caption.as_ptr(),
            0x00000000 | 0x00000010,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_license_error(app_name: &str, detail: &str) {
    eprintln!(
        "No valid RELIQUARY license is activated for {app_name}. Open Obelisk and activate or refresh a Suite license. Details: {detail}"
    );
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

fn silent_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);

    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}
