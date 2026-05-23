use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env,
    ffi::OsStr,
    fs,
    io::{self, BufRead, BufReader, Cursor, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};
use tauri::{AppHandle, Emitter};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const DOWNLOAD_EVENT: &str = "daedalus://download-event";
const YT_DLP_STABLE_URL: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const YT_DLP_NIGHTLY_URL: &str =
    "https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp.exe";
const YT_DLP_MASTER_URL: &str =
    "https://github.com/yt-dlp/yt-dlp-master-builds/releases/latest/download/yt-dlp.exe";
const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
const DENO_URL: &str =
    "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Serialize)]
pub struct ToolStatus {
    name: String,
    installed: bool,
    managed: bool,
    path: Option<String>,
    version: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SystemStatus {
    yt_dlp: ToolStatus,
    ffmpeg: ToolStatus,
    deno: ToolStatus,
    default_output_dir: String,
    tools_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct InstallToolRequest {
    tool: String,
    channel: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProbeRequest {
    url: String,
    include_playlist: bool,
}

#[derive(Debug, Serialize)]
pub struct SourceMetadata {
    id: Option<String>,
    title: Option<String>,
    uploader: Option<String>,
    webpage_url: Option<String>,
    extractor: Option<String>,
    duration_seconds: Option<f64>,
    thumbnail: Option<String>,
    is_live: bool,
    entry_count: Option<usize>,
    formats: Vec<MediaFormat>,
}

#[derive(Debug, Serialize)]
pub struct MediaFormat {
    format_id: Option<String>,
    ext: Option<String>,
    resolution: Option<String>,
    fps: Option<f64>,
    filesize: Option<u64>,
    vcodec: Option<String>,
    acodec: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    url: String,
    output_dir: String,
    mode: String,
    quality: String,
    video_format: String,
    audio_format: String,
    include_playlist: bool,
    embed_metadata: bool,
    embed_thumbnail: bool,
    write_subtitles: bool,
    write_auto_subtitles: bool,
    #[serde(default)]
    embed_chapters: bool,
    avoid_redownload: bool,
    concurrent_fragments: u8,
    skip_unavailable: bool,
    ignore_errors: bool,
    restrict_filenames: bool,
    prefer_free_formats: bool,
    no_check_certificate: bool,
    write_info_json: bool,
    keep_intermediate: bool,
    #[serde(default)]
    filename_template: String,
    #[serde(default)]
    extra_args: String,
    #[serde(default)]
    write_description: bool,
    #[serde(default)]
    write_thumbnail: bool,
    #[serde(default)]
    write_comments: bool,
    #[serde(default)]
    write_playlist_metadata: bool,
    #[serde(default)]
    mark_watched: bool,
    #[serde(default)]
    remove_sponsor_segments: bool,
    #[serde(default)]
    live_from_start: bool,
    #[serde(default)]
    verbose_logs: bool,
    #[serde(default)]
    cookie_browser: String,
    #[serde(default)]
    network_stack: String,
    #[serde(default)]
    rate_limit: String,
    #[serde(default = "default_retry_count")]
    retry_count: u8,
    #[serde(default = "default_retry_count")]
    fragment_retry_count: u8,
    #[serde(default)]
    sleep_requests: u8,
}

#[derive(Debug, Serialize, Clone)]
pub struct DownloadEvent {
    kind: String,
    stream: Option<String>,
    message: String,
    progress: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct DownloadResult {
    success: bool,
    code: Option<i32>,
    output_tail: Vec<String>,
}

#[tauri::command]
pub async fn get_system_status() -> Result<SystemStatus, String> {
    tauri::async_runtime::spawn_blocking(system_status)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn open_obelisk() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(launch_obelisk)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn install_tool(request: InstallToolRequest) -> Result<SystemStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let yt_dlp_channel = request.channel.as_deref().unwrap_or("stable");

        match request.tool.as_str() {
            "yt-dlp" => install_yt_dlp(yt_dlp_channel)?,
            "ffmpeg" => install_ffmpeg()?,
            "deno" => install_deno()?,
            "all" => {
                install_yt_dlp(yt_dlp_channel)?;
                install_ffmpeg()?;
                install_deno()?;
            }
            other => return Err(format!("Unsupported tool: {other}")),
        }

        system_status()
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn probe_source(request: ProbeRequest) -> Result<SourceMetadata, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_url(&request.url)?;

        let yt_dlp = resolve_tool("yt-dlp")?;
        let mut args = vec![
            "--dump-single-json".to_string(),
            "--no-warnings".to_string(),
            "--skip-download".to_string(),
        ];
        append_js_runtime_args(&mut args);

        if request.include_playlist {
            args.push("--yes-playlist".to_string());
        } else {
            args.push("--no-playlist".to_string());
        }

        args.push(request.url);

        let output = yt_dlp_command(&yt_dlp)
            .args(args)
            .output()
            .map_err(|error| format!("Unable to run yt-dlp: {error}"))?;

        if !output.status.success() {
            return Err(command_error(
                "yt-dlp probe failed",
                &output.stderr,
                &output.stdout,
            ));
        }

        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("Unable to parse yt-dlp metadata: {error}"))?;

        Ok(metadata_from_value(&value))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    request: DownloadRequest,
) -> Result<DownloadResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_download_request(&request)?;

        let yt_dlp = resolve_tool("yt-dlp")?;
        let args = build_download_args(&request)?;

        emit_event(
            &app,
            "started",
            None,
            format!("Starting yt-dlp with {} arguments", args.len()),
            None,
        );

        let mut child = yt_dlp_command(&yt_dlp)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("Unable to start yt-dlp: {error}"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Unable to read yt-dlp stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Unable to read yt-dlp stderr".to_string())?;

        let stdout_thread = pump_reader(stdout, app.clone(), "stdout");
        let stderr_thread = pump_reader(stderr, app.clone(), "stderr");

        let status = child
            .wait()
            .map_err(|error| format!("Unable to wait for yt-dlp: {error}"))?;

        let mut lines = stdout_thread.join().unwrap_or_default();
        lines.extend(stderr_thread.join().unwrap_or_default());
        let output_tail = trim_tail(lines, 240);
        let output_dir = normalize_output_dir(&request.output_dir)?;
        let temp_dir = temp_download_dir()?;
        let cleaned_files = cleanup_partial_files(&output_dir) + cleanup_partial_files(&temp_dir);

        if cleaned_files > 0 {
            emit_event(
                &app,
                "log",
                Some("stdout"),
                format!("Cleaned {cleaned_files} temporary download file(s)"),
                None,
            );
        }

        if status.success() {
            emit_event(
                &app,
                "completed",
                None,
                "Download completed".to_string(),
                Some(100.0),
            );
        } else {
            emit_event(
                &app,
                "failed",
                None,
                format!("yt-dlp exited with code {:?}", status.code()),
                None,
            );
        }

        Ok(DownloadResult {
            success: status.success(),
            code: status.code(),
            output_tail,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

fn system_status() -> Result<SystemStatus, String> {
    Ok(SystemStatus {
        yt_dlp: inspect_tool("yt-dlp"),
        ffmpeg: inspect_tool("ffmpeg"),
        deno: inspect_tool("deno"),
        default_output_dir: default_output_dir(),
        tools_dir: tools_bin_dir()?.to_string_lossy().to_string(),
    })
}

fn inspect_tool(name: &str) -> ToolStatus {
    match resolve_tool(name) {
        Ok(path) => {
            let version = silent_command(&path)
                .arg("--version")
                .output()
                .ok()
                .and_then(|output| {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    text.lines()
                        .next()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .map(ToOwned::to_owned)
                });

            ToolStatus {
                name: name.to_string(),
                installed: true,
                managed: is_managed_tool_path(&path),
                path: Some(path.to_string_lossy().to_string()),
                version,
                error: None,
            }
        }
        Err(error) => ToolStatus {
            name: name.to_string(),
            installed: false,
            managed: false,
            path: None,
            version: None,
            error: Some(error),
        },
    }
}

fn resolve_tool(name: &str) -> Result<PathBuf, String> {
    for managed in managed_tool_candidates(name) {
        if managed.exists() {
            return Ok(managed);
        }
    }

    if let Ok(path) = which::which(name) {
        return Ok(path);
    }

    if let Some(path) = find_common_windows_tool(name) {
        return Ok(path);
    }

    Err(format!("{name} was not found"))
}

fn yt_dlp_command(path: &Path) -> Command {
    let mut command = silent_command(path);
    command.env("PYTHONIOENCODING", "utf-8");
    command.env("PYTHONUTF8", "1");
    command
}

fn install_yt_dlp(channel: &str) -> Result<(), String> {
    ensure_windows_install_supported()?;

    let bin_dir = tools_bin_dir()?;
    fs::create_dir_all(&bin_dir)
        .map_err(|error| format!("Unable to create tools directory: {error}"))?;

    let destination = bin_dir.join("yt-dlp.exe");
    let temp_destination = bin_dir.join("yt-dlp.exe.download");
    let bytes = download_bytes(yt_dlp_download_url(channel)?)?;

    fs::write(&temp_destination, bytes)
        .map_err(|error| format!("Unable to write yt-dlp: {error}"))?;
    replace_file(&temp_destination, &destination)?;

    Ok(())
}

fn yt_dlp_download_url(channel: &str) -> Result<&'static str, String> {
    match channel.trim().to_ascii_lowercase().as_str() {
        "" | "stable" | "release" => Ok(YT_DLP_STABLE_URL),
        "nightly" => Ok(YT_DLP_NIGHTLY_URL),
        "master" => Ok(YT_DLP_MASTER_URL),
        other => Err(format!("Unsupported yt-dlp update channel: {other}")),
    }
}

fn install_ffmpeg() -> Result<(), String> {
    ensure_windows_install_supported()?;

    let bin_dir = tools_bin_dir()?;
    fs::create_dir_all(&bin_dir)
        .map_err(|error| format!("Unable to create tools directory: {error}"))?;

    let bytes = download_bytes(FFMPEG_URL)?;
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| format!("Unable to read ffmpeg archive: {error}"))?;
    let mut installed = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Unable to read ffmpeg archive entry: {error}"))?;
        let normalized_name = entry.name().replace('\\', "/").to_lowercase();

        if !(normalized_name.ends_with("/bin/ffmpeg.exe")
            || normalized_name.ends_with("/bin/ffprobe.exe"))
        {
            continue;
        }

        let file_name = normalized_name
            .rsplit('/')
            .next()
            .ok_or_else(|| "Unable to read ffmpeg archive file name".to_string())?;
        let temp_destination = bin_dir.join(format!("{file_name}.download"));
        let destination = bin_dir.join(file_name);
        let mut output = fs::File::create(&temp_destination)
            .map_err(|error| format!("Unable to write {file_name}: {error}"))?;

        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("Unable to extract {file_name}: {error}"))?;
        drop(output);
        replace_file(&temp_destination, &destination)?;
        installed.push(file_name.to_string());
    }

    if !installed.iter().any(|name| name == "ffmpeg.exe") {
        return Err("The downloaded ffmpeg archive did not contain ffmpeg.exe".to_string());
    }

    Ok(())
}

fn install_deno() -> Result<(), String> {
    ensure_windows_install_supported()?;

    let bin_dir = tools_bin_dir()?;
    fs::create_dir_all(&bin_dir)
        .map_err(|error| format!("Unable to create tools directory: {error}"))?;

    let bytes = download_bytes(DENO_URL)?;
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| format!("Unable to read Deno archive: {error}"))?;
    let mut installed = false;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Unable to read Deno archive entry: {error}"))?;
        let normalized_name = entry.name().replace('\\', "/").to_lowercase();

        if !normalized_name.ends_with("deno.exe") {
            continue;
        }

        let temp_destination = bin_dir.join("deno.exe.download");
        let destination = bin_dir.join("deno.exe");
        let mut output = fs::File::create(&temp_destination)
            .map_err(|error| format!("Unable to write deno.exe: {error}"))?;

        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("Unable to extract deno.exe: {error}"))?;
        drop(output);
        replace_file(&temp_destination, &destination)?;
        installed = true;
        break;
    }

    if !installed {
        return Err("The downloaded Deno archive did not contain deno.exe".to_string());
    }

    Ok(())
}

fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Daedalus/0.1 RELIQUARY")
        .build()
        .map_err(|error| format!("Unable to create download client: {error}"))?;
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("Unable to download {url}: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Unable to download {url}: {error}"))?;

    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| format!("Unable to read downloaded file: {error}"))
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        fs::remove_file(destination)
            .map_err(|error| format!("Unable to replace {}: {error}", destination.display()))?;
    }

    fs::rename(source, destination).map_err(|error| {
        format!(
            "Unable to move {} into place: {error}",
            destination.display()
        )
    })
}

fn ensure_windows_install_supported() -> Result<(), String> {
    if cfg!(windows) {
        Ok(())
    } else {
        Err("Managed tool installation is currently available on Windows only.".to_string())
    }
}

fn legacy_managed_tool_path(name: &str) -> Result<PathBuf, String> {
    Ok(app_data_dir()?
        .join("tools")
        .join("bin")
        .join(executable_name(name)))
}

fn managed_tool_candidates(name: &str) -> Vec<PathBuf> {
    let executable = executable_name(name);
    managed_tool_bin_dirs()
        .into_iter()
        .map(|path| path.join(&executable))
        .collect()
}

fn managed_tool_bin_dirs() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = tools_bin_dir() {
        candidates.push(path);
    }

    if let Some(root) = reliquary_workspace_root() {
        candidates.push(root.join("toolchain").join("bin"));
    }

    if let Some(root) = reliquary_install_root() {
        candidates.push(root.join("toolchain").join("bin"));
    }

    if let Ok(root) = user_reliquary_root() {
        candidates.push(root.join("toolchain").join("bin"));
    }

    if let Ok(base) = env::var("ProgramFiles") {
        candidates.push(
            PathBuf::from(base)
                .join("Reliquary")
                .join("toolchain")
                .join("bin"),
        );
    }

    if let Ok(path) = legacy_managed_tool_path("placeholder") {
        if let Some(bin_dir) = path.parent() {
            candidates.push(bin_dir.to_path_buf());
        }
    }

    candidates
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn is_managed_tool_path(path: &Path) -> bool {
    managed_tool_bin_dirs()
        .into_iter()
        .any(|tools_dir| path.starts_with(tools_dir))
}

fn tools_bin_dir() -> Result<PathBuf, String> {
    if let Ok(toolchain_dir) = env::var("RELIQUARY_TOOLCHAIN_DIR") {
        let toolchain_dir = toolchain_dir.trim();
        if !toolchain_dir.is_empty() {
            return Ok(PathBuf::from(toolchain_dir).join("bin"));
        }
    }

    if let Some(root) = reliquary_workspace_root() {
        return Ok(root.join("toolchain").join("bin"));
    }

    if let Some(root) = reliquary_install_root() {
        return Ok(root.join("toolchain").join("bin"));
    }

    Ok(user_reliquary_root()?.join("toolchain").join("bin"))
}

fn download_archive_path() -> Result<PathBuf, String> {
    let archive_dir = app_data_dir()?.join("archives");
    fs::create_dir_all(&archive_dir)
        .map_err(|error| format!("Unable to create archive directory: {error}"))?;

    Ok(archive_dir.join("downloads.txt"))
}

fn temp_download_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("temp"))
}

fn remove_legacy_archive_files(output_dir: &Path) {
    for file_name in ["daedalus-archive.txt", ".daedalus-archive.txt"] {
        let _ = fs::remove_file(output_dir.join(file_name));
    }
}

fn app_data_dir() -> Result<PathBuf, String> {
    Ok(user_reliquary_root()?.join("Daedalus"))
}

fn user_reliquary_root() -> Result<PathBuf, String> {
    let base_dir = env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|_| env::var("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".local").join("share")))
        .map_err(|_| "Unable to find a user data directory for Daedalus tools".to_string())?;

    Ok(base_dir.join("RELIQUARY"))
}

fn normalize_output_dir(raw_path: &str) -> Result<PathBuf, String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("An output directory is required".to_string());
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(home_dir().join(path))
    }
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn validate_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("A URL is required".to_string());
    }

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("The source must be an http or https URL".to_string());
    }

    Ok(())
}

fn validate_download_request(request: &DownloadRequest) -> Result<(), String> {
    validate_url(&request.url)?;
    resolve_tool("ffmpeg").map_err(|error| format!("{error}. Install ffmpeg from Settings."))?;

    let output_dir = normalize_output_dir(&request.output_dir)?;
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .map_err(|error| format!("Unable to create output directory: {error}"))?;
    }

    if !output_dir.is_dir() {
        return Err("The output path is not a directory".to_string());
    }

    remove_legacy_archive_files(&output_dir);

    Ok(())
}

fn build_download_args(request: &DownloadRequest) -> Result<Vec<String>, String> {
    let output_dir = normalize_output_dir(&request.output_dir)?;
    let temp_dir = temp_download_dir()?;
    fs::create_dir_all(&temp_dir)
        .map_err(|error| format!("Unable to create temp download directory: {error}"))?;

    let mut args = vec![
        "--newline".to_string(),
        "--no-color".to_string(),
        "--encoding".to_string(),
        "utf-8".to_string(),
        "--windows-filenames".to_string(),
        "--no-part".to_string(),
        "--no-mtime".to_string(),
        "--trim-filenames".to_string(),
        "155".to_string(),
        "--concurrent-fragments".to_string(),
        request.concurrent_fragments.clamp(1, 16).to_string(),
        "--retries".to_string(),
        request.retry_count.clamp(0, 100).to_string(),
        "--fragment-retries".to_string(),
        request.fragment_retry_count.clamp(0, 100).to_string(),
        "--paths".to_string(),
        format!("home:{}", yt_dlp_path_arg(&output_dir)),
        "--paths".to_string(),
        format!("temp:{}", yt_dlp_path_arg(&temp_dir)),
    ];

    args.extend([
        "-o".to_string(),
        output_template(&request.filename_template, request.include_playlist),
    ]);

    if request.include_playlist {
        args.push("--yes-playlist".to_string());
    } else {
        args.push("--no-playlist".to_string());
    }

    if request.ignore_errors {
        args.push("--ignore-errors".to_string());
    } else if request.skip_unavailable {
        args.push("--no-abort-on-error".to_string());
    } else {
        args.push("--abort-on-error".to_string());
    }

    if request.restrict_filenames {
        args.push("--restrict-filenames".to_string());
    }

    if request.prefer_free_formats {
        args.push("--prefer-free-formats".to_string());
    }

    if request.no_check_certificate {
        args.push("--no-check-certificates".to_string());
    }

    append_rate_limit_args(&mut args, &request.rate_limit)?;
    append_cookie_args(&mut args, &request.cookie_browser)?;
    append_network_args(&mut args, &request.network_stack)?;

    if request.sleep_requests > 0 {
        args.extend([
            "--sleep-requests".to_string(),
            request.sleep_requests.clamp(0, 60).to_string(),
        ]);
    }

    let ffmpeg_path = resolve_tool("ffmpeg")?;
    args.extend([
        "--ffmpeg-location".to_string(),
        ffmpeg_location(&ffmpeg_path).to_string_lossy().to_string(),
    ]);
    append_js_runtime_args(&mut args);

    if request.avoid_redownload {
        let archive = download_archive_path()?;
        args.extend([
            "--download-archive".to_string(),
            archive.to_string_lossy().to_string(),
        ]);
    }

    if request.write_info_json {
        args.push("--write-info-json".to_string());
    }

    if request.write_description {
        args.push("--write-description".to_string());
    }

    if request.write_thumbnail {
        args.push("--write-thumbnail".to_string());
    }

    if request.write_comments {
        args.push("--write-comments".to_string());
        if !request.write_info_json {
            args.push("--write-info-json".to_string());
        }
    }

    if request.write_playlist_metadata {
        args.push("--write-playlist-metafiles".to_string());
    }

    if request.keep_intermediate {
        args.push("--keep-video".to_string());
    }

    if request.embed_metadata {
        args.push("--embed-metadata".to_string());
    }

    if request.embed_thumbnail {
        args.push("--embed-thumbnail".to_string());
    }

    if request.write_subtitles {
        args.push("--write-subs".to_string());
        args.extend(["--sub-langs".to_string(), "all,-live_chat".to_string()]);
    }

    if request.write_auto_subtitles {
        args.push("--write-auto-subs".to_string());
        if !request.write_subtitles {
            args.extend(["--sub-langs".to_string(), "all,-live_chat".to_string()]);
        }
    }

    if request.embed_chapters {
        args.push("--embed-chapters".to_string());
    }

    if request.mark_watched {
        args.push("--mark-watched".to_string());
    }

    if request.remove_sponsor_segments {
        args.extend(["--sponsorblock-remove".to_string(), "all".to_string()]);
    }

    if request.live_from_start {
        args.push("--live-from-start".to_string());
    }

    if request.verbose_logs {
        args.push("--verbose".to_string());
    }

    match request.mode.as_str() {
        "audio" => {
            args.extend(["-f".to_string(), "ba/bestaudio".to_string()]);
            args.push("--extract-audio".to_string());
            args.extend(["--audio-format".to_string(), request.audio_format.clone()]);
            args.extend(["--audio-quality".to_string(), "0".to_string()]);
        }
        "video" => {
            args.extend([
                "-f".to_string(),
                format_selector(&request.quality, &request.video_format),
            ]);

            if request.video_format != "source" {
                args.extend([
                    "--merge-output-format".to_string(),
                    request.video_format.clone(),
                    "--remux-video".to_string(),
                    request.video_format.clone(),
                ]);
            }
        }
        other => return Err(format!("Unsupported download mode: {other}")),
    }

    args.extend(parse_extra_args(&request.extra_args)?);
    args.push(request.url.trim().to_string());
    Ok(args)
}

fn output_template(raw_template: &str, include_playlist: bool) -> String {
    let fallback = if include_playlist {
        "{PLAYLIST_INDEX} - {TITLE}.{FILE_EXTENSION}"
    } else {
        "{TITLE}.{FILE_EXTENSION}"
    };
    let raw_template = raw_template.trim();
    let source = if raw_template.is_empty() {
        fallback
    } else {
        raw_template
    };
    let mut template = source.to_string();
    let replacements = [
        ("{TITLE}", "%(title)s"),
        ("{FULL_TITLE}", "%(fulltitle)s"),
        ("{FILE_EXTENSION}", "%(ext)s"),
        ("{EXT}", "%(ext)s"),
        ("{UPLOADER}", "%(uploader)s"),
        ("{UPLOADER_ID}", "%(uploader_id)s"),
        ("{ID}", "%(id)s"),
        ("{DESCRIPTION}", "%(description).160s"),
        ("{PLAYLIST_INDEX}", "%(playlist_index)03d"),
    ];

    for (tag, value) in replacements {
        template = template.replace(tag, value);
    }

    if !source.contains("{FILE_EXTENSION}")
        && !source.contains("{EXT}")
        && !template.contains("%(ext)")
    {
        template.push_str(".%(ext)s");
    }

    template
}

fn default_retry_count() -> u8 {
    10
}

fn append_rate_limit_args(args: &mut Vec<String>, rate_limit: &str) -> Result<(), String> {
    let normalized = rate_limit.trim();
    if normalized.is_empty() || normalized.eq_ignore_ascii_case("none") {
        return Ok(());
    }

    const ALLOWED_RATE_LIMITS: [&str; 5] = ["1M", "2M", "5M", "10M", "25M"];
    if !ALLOWED_RATE_LIMITS
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(normalized))
    {
        return Err(format!("Unsupported rate limit: {normalized}"));
    }

    args.extend(["--limit-rate".to_string(), normalized.to_ascii_uppercase()]);
    Ok(())
}

fn append_cookie_args(args: &mut Vec<String>, browser: &str) -> Result<(), String> {
    let normalized = browser.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "none" {
        return Ok(());
    }

    const ALLOWED_BROWSERS: [&str; 6] = ["firefox", "chrome", "edge", "brave", "opera", "vivaldi"];
    if !ALLOWED_BROWSERS.contains(&normalized.as_str()) {
        return Err(format!("Unsupported browser cookie source: {normalized}"));
    }

    args.extend(["--cookies-from-browser".to_string(), normalized]);
    Ok(())
}

fn append_network_args(args: &mut Vec<String>, network_stack: &str) -> Result<(), String> {
    match network_stack.trim().to_ascii_lowercase().as_str() {
        "" | "auto" => Ok(()),
        "ipv4" => {
            args.push("--force-ipv4".to_string());
            Ok(())
        }
        "ipv6" => {
            args.push("--force-ipv6".to_string());
            Ok(())
        }
        other => Err(format!("Unsupported network stack: {other}")),
    }
}

fn parse_extra_args(raw_args: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for character in raw_args.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        if character == '\\' && quote.is_some() {
            escaped = true;
            continue;
        }

        if let Some(active_quote) = quote {
            if character == active_quote {
                quote = None;
            } else {
                current.push(character);
            }
            continue;
        }

        match character {
            '"' | '\'' => quote = Some(character),
            character if character.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
    }

    if escaped {
        current.push('\\');
    }

    if quote.is_some() {
        return Err("Extra yt-dlp arguments contain an unclosed quote".to_string());
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

fn ffmpeg_location(path: &Path) -> PathBuf {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case("ffmpeg.exe") || name.eq_ignore_ascii_case("ffmpeg")
        })
    {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

fn append_js_runtime_args(args: &mut Vec<String>) {
    if let Ok(deno_path) = resolve_tool("deno") {
        args.extend([
            "--js-runtimes".to_string(),
            format!("deno:{}", yt_dlp_path_arg(&deno_path)),
        ]);
    }
}

fn yt_dlp_path_arg(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn cleanup_partial_files(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            let is_temporary_download = path.is_file()
                && (file_name.ends_with(".part")
                    || file_name.ends_with(".ytdl")
                    || file_name.ends_with(".temp"));

            if is_temporary_download {
                Some(path)
            } else {
                None
            }
        })
        .filter(|path| fs::remove_file(path).is_ok())
        .count()
}

fn find_common_windows_tool(name: &str) -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }

    let executable = executable_name(name);
    let mut candidates = Vec::new();

    if name == "ffmpeg" {
        candidates.push(PathBuf::from(r"C:\ffmpeg\bin").join(&executable));
        candidates.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin").join(&executable));
        candidates.push(PathBuf::from(r"C:\Program Files\ffmpeg\bin").join(&executable));

        if let Some(found) = find_executable_under(&PathBuf::from(r"C:\ffmpeg"), &executable, 4) {
            return Some(found);
        }
    } else if name == "deno" {
        if let Ok(home) = env::var("USERPROFILE") {
            let home = PathBuf::from(home);
            candidates.push(home.join(".deno").join("bin").join(&executable));
            candidates.push(home.join("scoop").join("shims").join(&executable));
        }

        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local_app_data)
                    .join("Microsoft")
                    .join("WinGet")
                    .join("Links")
                    .join(&executable),
            );
        }

        candidates.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin").join(&executable));
    }

    candidates.into_iter().find(|path| path.exists())
}

fn launch_obelisk() -> Result<(), String> {
    let executable = resolve_obelisk_executable()
        .ok_or_else(|| "Obelisk is not built or installed yet.".to_string())?;

    silent_command(&executable)
        .spawn()
        .map_err(|error| format!("Unable to launch Obelisk: {error}"))?;

    Ok(())
}

fn resolve_obelisk_executable() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(root) = reliquary_workspace_root() {
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

    if let Some(root) = reliquary_install_root() {
        candidates.push(root.join("obelisk").join("obelisk.exe"));
        candidates.push(root.join("Obelisk").join("obelisk.exe"));
    }

    for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(base) = env::var(key) {
            let base = PathBuf::from(base);
            candidates.push(base.join("Obelisk").join("obelisk.exe"));
            candidates.push(base.join("RELIQUARY").join("obelisk").join("obelisk.exe"));
            candidates.push(base.join("RELIQUARY").join("Obelisk").join("obelisk.exe"));
            candidates.push(base.join("Programs").join("Obelisk").join("obelisk.exe"));
            candidates.push(
                base.join("Programs")
                    .join("RELIQUARY")
                    .join("obelisk")
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

fn reliquary_workspace_root() -> Option<PathBuf> {
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
            if ancestor.join("obelisk").is_dir() && ancestor.join("daedalus").is_dir() {
                return Some(ancestor.to_path_buf());
            }
        }
    }

    None
}

fn reliquary_install_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let app_dir = exe.parent()?;
    let root = app_dir.parent()?;
    let app_name = app_dir.file_name()?.to_string_lossy().to_ascii_lowercase();

    matches!(app_name.as_str(), "daedalus" | "chronos" | "obelisk").then(|| root.to_path_buf())
}

fn silent_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);

    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

fn find_executable_under(root: &Path, executable: &str, max_depth: usize) -> Option<PathBuf> {
    if max_depth == 0 || !root.is_dir() {
        return None;
    }

    let direct = root.join(executable);
    if direct.exists() {
        return Some(direct);
    }

    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("node_modules") {
                continue;
            }

            if let Some(found) = find_executable_under(&path, executable, max_depth - 1) {
                return Some(found);
            }
        }
    }

    None
}

fn format_selector(quality: &str, container: &str) -> String {
    if quality == "small" {
        return "worstvideo+worstaudio/worst".to_string();
    }

    let height_filter = match quality {
        "2160" | "1440" | "1080" | "720" | "480" => format!("[height<={quality}]"),
        _ => String::new(),
    };

    match container {
        "mp4" | "mov" => format!(
            "bv*{height_filter}[ext=mp4]+ba[ext=m4a]/b{height_filter}[ext=mp4]/bv*{height_filter}+ba/b{height_filter}"
        ),
        "webm" => format!(
            "bv*{height_filter}[ext=webm]+ba[ext=webm]/b{height_filter}[ext=webm]/bv*{height_filter}+ba/b{height_filter}"
        ),
        _ => format!("bv*{height_filter}+ba/b{height_filter}"),
    }
}

fn metadata_from_value(value: &Value) -> SourceMetadata {
    let formats = value
        .get("formats")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .rev()
                .take(80)
                .map(|format| MediaFormat {
                    format_id: string_field(format, "format_id"),
                    ext: string_field(format, "ext"),
                    resolution: string_field(format, "resolution").or_else(|| {
                        let width = format.get("width").and_then(Value::as_u64);
                        let height = format.get("height").and_then(Value::as_u64);
                        match (width, height) {
                            (Some(width), Some(height)) => Some(format!("{width}x{height}")),
                            (_, Some(height)) => Some(format!("{height}p")),
                            _ => None,
                        }
                    }),
                    fps: format.get("fps").and_then(Value::as_f64),
                    filesize: format
                        .get("filesize")
                        .or_else(|| format.get("filesize_approx"))
                        .and_then(Value::as_u64),
                    vcodec: string_field(format, "vcodec"),
                    acodec: string_field(format, "acodec"),
                })
                .collect()
        })
        .unwrap_or_default();

    SourceMetadata {
        id: string_field(value, "id"),
        title: string_field(value, "title"),
        uploader: string_field(value, "uploader").or_else(|| string_field(value, "channel")),
        webpage_url: string_field(value, "webpage_url"),
        extractor: string_field(value, "extractor_key")
            .or_else(|| string_field(value, "extractor")),
        duration_seconds: value.get("duration").and_then(Value::as_f64),
        thumbnail: string_field(value, "thumbnail"),
        is_live: value
            .get("is_live")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        entry_count: value
            .get("entries")
            .and_then(Value::as_array)
            .map(Vec::len)
            .or_else(|| {
                value
                    .get("playlist_count")
                    .and_then(Value::as_u64)
                    .map(|count| count as usize)
            }),
        formats,
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn pump_reader<R: Read + Send + 'static>(
    reader: R,
    app: AppHandle,
    stream: &'static str,
) -> thread::JoinHandle<Vec<String>> {
    thread::spawn(move || {
        let mut lines = Vec::new();
        let reader = BufReader::new(reader);

        for line in reader.lines().map_while(Result::ok) {
            let progress = parse_progress(&line);
            emit_event(
                &app,
                if progress.is_some() {
                    "progress"
                } else {
                    "log"
                },
                Some(stream),
                line.clone(),
                progress,
            );
            lines.push(line);
        }

        lines
    })
}

fn parse_progress(line: &str) -> Option<f32> {
    let percent_index = line.find('%')?;
    let before_percent = &line[..percent_index];
    let start = before_percent
        .rfind(|character: char| !(character.is_ascii_digit() || character == '.'))
        .map(|index| index + 1)
        .unwrap_or(0);

    before_percent[start..].trim().parse::<f32>().ok()
}

fn emit_event(
    app: &AppHandle,
    kind: &str,
    stream: Option<&str>,
    message: String,
    progress: Option<f32>,
) {
    let _ = app.emit(
        DOWNLOAD_EVENT,
        DownloadEvent {
            kind: kind.to_string(),
            stream: stream.map(ToOwned::to_owned),
            message,
            progress,
        },
    );
}

fn command_error(prefix: &str, stderr: &[u8], stdout: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();

    if !stderr.is_empty() {
        format!("{prefix}: {stderr}")
    } else if !stdout.is_empty() {
        format!("{prefix}: {stdout}")
    } else {
        prefix.to_string()
    }
}

fn trim_tail(mut lines: Vec<String>, max_lines: usize) -> Vec<String> {
    if lines.len() <= max_lines {
        lines
    } else {
        lines.split_off(lines.len() - max_lines)
    }
}

fn default_output_dir() -> String {
    home_dir()
        .join("Downloads")
        .join("Daedalus")
        .to_string_lossy()
        .to_string()
}
