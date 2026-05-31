import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { installReliquaryThemeSync } from "./theme";
import {
  Activity,
  AlertCircle,
  CheckCircle2,
  Download,
  Film,
  FolderInput,
  FolderOpen,
  ListVideo,
  Music2,
  PackageCheck,
  RefreshCcw,
  Settings,
  SlidersHorizontal,
  Sparkles,
  Tags,
  Terminal,
  Wrench,
  X,
} from "lucide-solid";
import { For, Match, Show, Switch, createEffect, createMemo, createSignal, onCleanup, onMount, type JSX } from "solid-js";

type ToolStatus = {
  name: string;
  installed: boolean;
  managed: boolean;
  path?: string;
  version?: string;
  error?: string;
};

type SystemStatus = {
  yt_dlp: ToolStatus;
  ffmpeg: ToolStatus;
  deno: ToolStatus;
  default_output_dir: string;
  tools_dir: string;
};

type SourceMetadata = {
  title?: string;
  uploader?: string;
  entry_count?: number;
};

type DownloadEvent = {
  kind: "started" | "progress" | "log" | "completed" | "failed";
  stream?: string;
  message: string;
  progress?: number;
};

type DownloadResult = {
  success: boolean;
  code?: number;
  output_tail: string[];
};

type QueueItem = {
  id: string;
  url: string;
  title: string;
  status: "queued" | "running" | "done" | "failed";
  progress: number;
};

type LogLine = {
  id: string;
  text: string;
  stream?: string;
};

type ToolId = "yt-dlp" | "ffmpeg" | "deno" | "all";
type SettingsCategory = "general" | "downloads" | "playlists" | "files" | "tools";
type NetworkStack = "auto" | "ipv4" | "ipv6";
type ThemeMode = "light" | "dark";
type DownloadMode = "video" | "audio";

type DaedalusSettings = {
  theme_mode: ThemeMode;
  output_dir: string;
  mode: DownloadMode;
  quality: string;
  video_format: string;
  audio_format: string;
  include_playlist: boolean;
  embed_metadata: boolean;
  embed_thumbnail: boolean;
  write_subtitles: boolean;
  embed_chapters: boolean;
  avoid_redownload: boolean;
  concurrent_fragments: number;
  skip_unavailable: boolean;
  ignore_errors: boolean;
  restrict_filenames: boolean;
  prefer_free_formats: boolean;
  no_check_certificate: boolean;
  write_info_json: boolean;
  keep_intermediate: boolean;
  filename_template: string;
  write_description: boolean;
  write_thumbnail_file: boolean;
  write_comments: boolean;
  write_playlist_metadata: boolean;
  mark_watched: boolean;
  remove_sponsor_segments: boolean;
  live_from_start: boolean;
  verbose_logs: boolean;
  cookie_browser: (typeof cookieBrowsers)[number];
  network_stack: NetworkStack;
  rate_limit: string;
  retry_count: number;
  fragment_retry_count: number;
  sleep_requests: number;
  notify_on_complete: boolean;
  console_height: number;
};

const qualityOptions = [
  { value: "best", label: "Best" },
  { value: "2160", label: "2160p" },
  { value: "1440", label: "1440p" },
  { value: "1080", label: "1080p" },
  { value: "720", label: "720p" },
  { value: "480", label: "480p" },
  { value: "small", label: "Smallest" },
];

const videoFormats = ["source", "mp4", "mkv", "webm", "mov"];
const audioFormats = ["mp3", "m4a", "flac", "wav", "opus"];
const cookieBrowsers = ["none", "firefox", "chrome", "edge", "brave", "opera", "vivaldi"] as const;
const rateLimitOptions = [
  { value: "none", label: "No limit" },
  { value: "1M", label: "1 MB/s" },
  { value: "2M", label: "2 MB/s" },
  { value: "5M", label: "5 MB/s" },
  { value: "10M", label: "10 MB/s" },
  { value: "25M", label: "25 MB/s" },
];
const filenameTags = [
  { tag: "{TITLE}", label: "TITLE" },
  { tag: "{FILE_EXTENSION}", label: "FILE EXTENSION" },
  { tag: "{UPLOADER}", label: "UPLOADER" },
  { tag: "{ID}", label: "ID" },
  { tag: "{FULL_TITLE}", label: "FULL TITLE" },
  { tag: "{DESCRIPTION}", label: "DESCRIPTION" },
  { tag: "{UPLOADER_ID}", label: "UPLOADER ID" },
  { tag: "{PLAYLIST_INDEX}", label: "PLAYLIST INDEX" },
];

function App() {
  const stopThemeSync = installReliquaryThemeSync();
  onCleanup(stopThemeSync);

  const [systemStatus, setSystemStatus] = createSignal<SystemStatus>();
  const [urlInput, setUrlInput] = createSignal("");
  const [outputDir, setOutputDir] = createSignal("");
  const [mode, setMode] = createSignal<DownloadMode>("video");
  const [quality, setQuality] = createSignal("best");
  const [videoFormat, setVideoFormat] = createSignal("mp4");
  const [audioFormat, setAudioFormat] = createSignal("mp3");
  const [includePlaylist, setIncludePlaylist] = createSignal(false);
  const [embedMetadata, setEmbedMetadata] = createSignal(true);
  const [embedThumbnail, setEmbedThumbnail] = createSignal(true);
  const [writeSubtitles, setWriteSubtitles] = createSignal(false);
  const [embedChapters, setEmbedChapters] = createSignal(true);
  const [avoidRedownload, setAvoidRedownload] = createSignal(false);
  const [concurrentFragments, setConcurrentFragments] = createSignal(4);
  const [skipUnavailable, setSkipUnavailable] = createSignal(true);
  const [ignoreErrors, setIgnoreErrors] = createSignal(false);
  const [restrictFilenames, setRestrictFilenames] = createSignal(true);
  const [preferFreeFormats, setPreferFreeFormats] = createSignal(false);
  const [noCheckCertificate, setNoCheckCertificate] = createSignal(false);
  const [writeInfoJson, setWriteInfoJson] = createSignal(false);
  const [keepIntermediate, setKeepIntermediate] = createSignal(false);
  const [filenameTemplate, setFilenameTemplate] = createSignal("{TITLE}.{FILE_EXTENSION}");
  const [writeDescription, setWriteDescription] = createSignal(false);
  const [writeThumbnailFile, setWriteThumbnailFile] = createSignal(false);
  const [writeComments, setWriteComments] = createSignal(false);
  const [writePlaylistMetadata, setWritePlaylistMetadata] = createSignal(false);
  const [markWatched, setMarkWatched] = createSignal(false);
  const [removeSponsorSegments, setRemoveSponsorSegments] = createSignal(false);
  const [liveFromStart, setLiveFromStart] = createSignal(false);
  const [verboseLogs, setVerboseLogs] = createSignal(false);
  const [cookieBrowser, setCookieBrowser] = createSignal<(typeof cookieBrowsers)[number]>("none");
  const [networkStack, setNetworkStack] = createSignal<NetworkStack>("auto");
  const [rateLimit, setRateLimit] = createSignal("none");
  const [retryCount, setRetryCount] = createSignal(10);
  const [fragmentRetryCount, setFragmentRetryCount] = createSignal(10);
  const [sleepRequests, setSleepRequests] = createSignal(0);
  const [notifyOnComplete, setNotifyOnComplete] = createSignal(true);
  const [consoleHeight, setConsoleHeight] = createSignal(150);
  const [queue, setQueue] = createSignal<QueueItem[]>([]);
  const [logs, setLogs] = createSignal<LogLine[]>([]);
  const [activeJobId, setActiveJobId] = createSignal<string>();
  const [isPreparingQueue, setIsPreparingQueue] = createSignal(false);
  const [isDownloading, setIsDownloading] = createSignal(false);
  const [installingTool, setInstallingTool] = createSignal<ToolId>();
  const [clearingArchive, setClearingArchive] = createSignal(false);
  const [settingsOpen, setSettingsOpen] = createSignal(false);
  const [settingsCategory, setSettingsCategory] = createSignal<SettingsCategory>("general");
  const [error, setError] = createSignal("");
  const [themeMode, setThemeModeState] = createSignal<ThemeMode>("light");
  let consoleOutputRef: HTMLDivElement | undefined;
  let settingsLoaded = false;

  const urls = createMemo(() =>
    urlInput()
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean),
  );

  const toolsReady = createMemo(() =>
    Boolean(systemStatus()?.yt_dlp.installed && systemStatus()?.ffmpeg.installed && systemStatus()?.deno.installed),
  );
  const missingTools = createMemo(() => {
    const status = systemStatus();
    if (!status) {
      return [];
    }

    return [
      status.yt_dlp.installed ? undefined : "yt-dlp",
      status.ffmpeg.installed ? undefined : "ffmpeg",
      status.deno.installed ? undefined : "deno",
    ].filter(Boolean) as Array<"yt-dlp" | "ffmpeg" | "deno">;
  });

  const canDownload = createMemo(
    () => urls().length > 0 && !isDownloading() && !isPreparingQueue() && Boolean(outputDir().trim()) && toolsReady(),
  );

  createEffect(() => {
    logs().length;
    queueMicrotask(() => {
      if (consoleOutputRef) {
        consoleOutputRef.scrollTop = consoleOutputRef.scrollHeight;
      }
    });
  });

  createEffect(() => {
    const settings = collectAppSettings();
    if (!settingsLoaded || !isTauriRuntime()) {
      return;
    }

    const timer = window.setTimeout(() => {
      void saveAppSettings(settings);
    }, 320);

    onCleanup(() => window.clearTimeout(timer));
  });

  onMount(async () => {
    await refreshStatus();
    await syncAppSettings();

    if (!isTauriRuntime()) {
      return;
    }

    const unlisten = await listen<DownloadEvent>("daedalus://download-event", (event) => {
      const payload = event.payload;
      appendLog(payload.message, payload.stream);

      if (payload.progress !== undefined) {
        updateActiveJob({ progress: Math.max(0, Math.min(100, payload.progress)) });
      }

      if (payload.kind === "completed") {
        updateActiveJob({ status: "done", progress: 100 });
      }

      if (payload.kind === "failed") {
        updateActiveJob({ status: "failed" });
      }
    });

    onCleanup(unlisten);
  });

  async function refreshStatus() {
    if (!isTauriRuntime()) {
      const status = {
        yt_dlp: {
          name: "yt-dlp",
          installed: false,
          managed: false,
          error: "Native bridge unavailable in browser preview.",
        },
        ffmpeg: {
          name: "ffmpeg",
          installed: false,
          managed: false,
          error: "Native bridge unavailable in browser preview.",
        },
        deno: {
          name: "deno",
          installed: false,
          managed: false,
          error: "Native bridge unavailable in browser preview.",
        },
        default_output_dir: "Downloads\\Daedalus",
        tools_dir: "System PATH / winget",
      };

      setSystemStatus(status);
      setOutputDir((current) => current || status.default_output_dir);
      return;
    }

    try {
      const status = await invoke<SystemStatus>("get_system_status");
      setSystemStatus(status);
      setOutputDir((current) => current || status.default_output_dir);
      setError("");
    } catch (caught) {
      setError(String(caught));
    }
  }

  async function installTool(tool: ToolId) {
    if (!isTauriRuntime()) {
      setError("Run Daedalus through Tauri to install system tools.");
      return;
    }

    setInstallingTool(tool);
    setError("");
    appendLog(`Installing ${tool} with the system package manager...`, "stdout");

    try {
      const status = await invoke<SystemStatus>("install_tool", {
        request: {
          tool,
          channel: tool === "yt-dlp" || tool === "all" ? "stable" : undefined,
        },
      });
      setSystemStatus(status);
      appendLog(`${tool} is ready.`, "stdout");
    } catch (caught) {
      setError(String(caught));
      appendLog(String(caught), "stderr");
    } finally {
      setInstallingTool(undefined);
    }
  }

  async function browseOutputDirectory() {
    if (!isTauriRuntime()) {
      setError("Run Daedalus through Tauri to browse folders.");
      return;
    }

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: outputDir(),
      });

      if (typeof selected === "string") {
        setOutputDir(selected);
      }
    } catch (caught) {
      setError(String(caught));
    }
  }

  async function syncAppSettings() {
    if (!isTauriRuntime()) {
      settingsLoaded = true;
      return;
    }

    try {
      const settings = await invoke<DaedalusSettings>("get_app_settings");
      applyAppSettings(settings);
    } catch {
      try {
        const mode = await invoke<string>("get_theme_mode");
        setThemeModeState(mode === "dark" ? "dark" : "light");
      } catch {
        setThemeModeState("light");
      }
    } finally {
      settingsLoaded = true;
    }
  }

  async function saveAppSettings(settings = collectAppSettings()) {
    try {
      const savedSettings = await invoke<DaedalusSettings>("set_app_settings", { settings });
      if (savedSettings.theme_mode !== themeMode()) {
        applyThemeMode(savedSettings.theme_mode);
      }
      setError("");
    } catch (caught) {
      setError(String(caught));
    }
  }

  async function changeThemeMode(mode: ThemeMode) {
    setThemeModeState(mode);
    applyThemeMode(mode);

    if (!isTauriRuntime()) {
      return;
    }

    try {
      const savedMode = await invoke<string>("set_theme_mode", { themeMode: mode });
      setThemeModeState(savedMode === "dark" ? "dark" : "light");
      setError("");
    } catch (caught) {
      setError(String(caught));
    }
  }

  function applyAppSettings(settings: DaedalusSettings) {
    const nextTheme = settings.theme_mode === "dark" ? "dark" : "light";
    applyThemeMode(nextTheme);
    if (settings.output_dir.trim()) {
      setOutputDir(settings.output_dir);
    }
    setMode(settings.mode === "audio" ? "audio" : "video");
    setQuality(settings.quality || "best");
    setVideoFormat(settings.video_format || "mp4");
    setAudioFormat(settings.audio_format || "mp3");
    setIncludePlaylist(settings.include_playlist);
    setEmbedMetadata(settings.embed_metadata);
    setEmbedThumbnail(settings.embed_thumbnail);
    setWriteSubtitles(settings.write_subtitles);
    setEmbedChapters(settings.embed_chapters);
    setAvoidRedownload(settings.avoid_redownload);
    setConcurrentFragments(clampNumber(settings.concurrent_fragments, 1, 16, 4));
    setSkipUnavailable(settings.skip_unavailable);
    setIgnoreErrors(settings.ignore_errors);
    setRestrictFilenames(settings.restrict_filenames);
    setPreferFreeFormats(settings.prefer_free_formats);
    setNoCheckCertificate(settings.no_check_certificate);
    setWriteInfoJson(settings.write_info_json);
    setKeepIntermediate(settings.keep_intermediate);
    setFilenameTemplate(settings.filename_template || "{TITLE}.{FILE_EXTENSION}");
    setWriteDescription(settings.write_description);
    setWriteThumbnailFile(settings.write_thumbnail_file);
    setWriteComments(settings.write_comments);
    setWritePlaylistMetadata(settings.write_playlist_metadata);
    setMarkWatched(settings.mark_watched);
    setRemoveSponsorSegments(settings.remove_sponsor_segments);
    setLiveFromStart(settings.live_from_start);
    setVerboseLogs(settings.verbose_logs);
    setCookieBrowser(cookieBrowsers.includes(settings.cookie_browser) ? settings.cookie_browser : "none");
    setNetworkStack(["auto", "ipv4", "ipv6"].includes(settings.network_stack) ? settings.network_stack : "auto");
    setRateLimit(settings.rate_limit || "none");
    setRetryCount(clampNumber(settings.retry_count, 0, 30, 10));
    setFragmentRetryCount(clampNumber(settings.fragment_retry_count, 0, 30, 10));
    setSleepRequests(clampNumber(settings.sleep_requests, 0, 10, 0));
    setNotifyOnComplete(settings.notify_on_complete);
    setConsoleHeight(clampNumber(settings.console_height, 96, 360, 150));
  }

  function collectAppSettings(): DaedalusSettings {
    return {
      theme_mode: themeMode(),
      output_dir: outputDir(),
      mode: mode(),
      quality: quality(),
      video_format: videoFormat(),
      audio_format: audioFormat(),
      include_playlist: includePlaylist(),
      embed_metadata: embedMetadata(),
      embed_thumbnail: embedThumbnail(),
      write_subtitles: writeSubtitles(),
      embed_chapters: embedChapters(),
      avoid_redownload: avoidRedownload(),
      concurrent_fragments: concurrentFragments(),
      skip_unavailable: skipUnavailable(),
      ignore_errors: ignoreErrors(),
      restrict_filenames: restrictFilenames(),
      prefer_free_formats: preferFreeFormats(),
      no_check_certificate: noCheckCertificate(),
      write_info_json: writeInfoJson(),
      keep_intermediate: keepIntermediate(),
      filename_template: filenameTemplate(),
      write_description: writeDescription(),
      write_thumbnail_file: writeThumbnailFile(),
      write_comments: writeComments(),
      write_playlist_metadata: writePlaylistMetadata(),
      mark_watched: markWatched(),
      remove_sponsor_segments: removeSponsorSegments(),
      live_from_start: liveFromStart(),
      verbose_logs: verboseLogs(),
      cookie_browser: cookieBrowser(),
      network_stack: networkStack(),
      rate_limit: rateLimit(),
      retry_count: retryCount(),
      fragment_retry_count: fragmentRetryCount(),
      sleep_requests: sleepRequests(),
      notify_on_complete: notifyOnComplete(),
      console_height: consoleHeight(),
    };
  }

  async function openAppFolder() {
    if (!isTauriRuntime()) {
      setError("Run Daedalus through Tauri to open the app folder.");
      return;
    }

    try {
      await invoke("open_app_folder");
      setError("");
    } catch (caught) {
      setError(String(caught));
    }
  }

  async function openToolchainFolder() {
    if (!isTauriRuntime()) {
      setError("Run Daedalus through Tauri to open the system tool location.");
      return;
    }

    try {
      await invoke("open_toolchain_folder");
      setError("");
    } catch (caught) {
      setError(String(caught));
    }
  }

  async function clearArchive() {
    if (!isTauriRuntime()) {
      setError("Run Daedalus through Tauri to clear the download archive.");
      return;
    }

    setClearingArchive(true);
    try {
      const cleared = await invoke<number>("clear_download_archive");
      appendLog(cleared > 0 ? `Cleared ${cleared} download archive file(s).` : "Download archive was already empty.", "stdout");
      setError("");
    } catch (caught) {
      setError(String(caught));
      appendLog(String(caught), "stderr");
    } finally {
      setClearingArchive(false);
    }
  }

  async function readTitle(url: string): Promise<string> {
    try {
      const metadata = await invoke<SourceMetadata>("probe_source", {
        request: {
          url,
          include_playlist: includePlaylist(),
        },
      });

      const title = metadata.entry_count && metadata.entry_count > 1
        ? `${metadata.title ?? labelFromUrl(url)} (${metadata.entry_count} items)`
        : metadata.title;

      return title || labelFromUrl(url);
    } catch (caught) {
      appendLog(`Could not read title for ${url}: ${String(caught)}`, "stderr");
      return labelFromUrl(url);
    }
  }

  async function prepareQueue() {
    setIsPreparingQueue(true);
    const prepared: QueueItem[] = [];

    try {
      for (const url of urls()) {
        prepared.push({
          id: crypto.randomUUID(),
          url,
          title: await readTitle(url),
          status: "queued",
          progress: 0,
        });
      }

      setQueue(prepared);
      return prepared;
    } finally {
      setIsPreparingQueue(false);
    }
  }

  async function downloadAll() {
    if (!isTauriRuntime()) {
      setError("Run Daedalus through Tauri to start downloads.");
      return;
    }

    if (!canDownload()) {
      return;
    }

    setError("");
    setLogs([]);

    const pending = await prepareQueue();
    if (pending.length === 0) {
      return;
    }

    setIsDownloading(true);
    let completedDownloads = 0;
    let failedDownloads = 0;

    for (const item of pending) {
      setActiveJobId(item.id);
      updateJob(item.id, { status: "running", progress: 0 });

      try {
        const result = await invoke<DownloadResult>("start_download", {
          request: buildDownloadRequest(item.url),
        });

        updateJob(item.id, {
          status: result.success ? "done" : "failed",
          progress: result.success ? 100 : queue().find((job) => job.id === item.id)?.progress ?? 0,
        });

        if (result.success) {
          completedDownloads += 1;
        } else {
          failedDownloads += 1;
        }
      } catch (caught) {
        failedDownloads += 1;
        updateJob(item.id, { status: "failed" });
        setError(String(caught));
        appendLog(String(caught), "stderr");
      }
    }

    setActiveJobId(undefined);
    setIsDownloading(false);
    await notifyDownloadComplete(completedDownloads, pending.length, failedDownloads);
  }

  async function notifyDownloadComplete(completed: number, total: number, failed: number) {
    if (!notifyOnComplete() || total === 0 || !("Notification" in window)) {
      return;
    }

    const body = failed > 0
      ? `${completed}/${total} downloads completed, ${failed} failed.`
      : `${completed}/${total} downloads completed.`;

    try {
      if (Notification.permission === "granted") {
        new Notification("Daedalus", { body });
      } else if (Notification.permission !== "denied") {
        const permission = await Notification.requestPermission();
        if (permission === "granted") {
          new Notification("Daedalus", { body });
        }
      }
    } catch (caught) {
      appendLog(`System notification unavailable: ${String(caught)}`, "stderr");
    }
  }

  function insertFilenameTag(tag: string) {
    setFilenameTemplate((current) => (current.trim() ? `${current} - ${tag}` : tag));
  }

  function startConsoleResize(event: PointerEvent) {
    event.preventDefault();
    const startY = event.clientY;
    const startHeight = consoleHeight();

    const onPointerMove = (moveEvent: PointerEvent) => {
      const nextHeight = startHeight + startY - moveEvent.clientY;
      setConsoleHeight(Math.max(96, Math.min(360, nextHeight)));
    };

    const onPointerUp = () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
  }

  function buildDownloadRequest(url: string) {
    return {
      url,
      output_dir: outputDir(),
      mode: mode(),
      quality: quality(),
      video_format: videoFormat(),
      audio_format: audioFormat(),
      include_playlist: includePlaylist(),
      embed_metadata: embedMetadata(),
      embed_thumbnail: embedThumbnail(),
      write_subtitles: writeSubtitles(),
      write_auto_subtitles: false,
      embed_chapters: embedChapters(),
      avoid_redownload: avoidRedownload(),
      concurrent_fragments: concurrentFragments(),
      skip_unavailable: skipUnavailable(),
      ignore_errors: ignoreErrors(),
      restrict_filenames: restrictFilenames(),
      prefer_free_formats: preferFreeFormats(),
      no_check_certificate: noCheckCertificate(),
      write_info_json: writeInfoJson(),
      keep_intermediate: keepIntermediate(),
      filename_template: filenameTemplate(),
      write_description: writeDescription(),
      write_thumbnail: writeThumbnailFile(),
      write_comments: writeComments(),
      write_playlist_metadata: writePlaylistMetadata(),
      mark_watched: markWatched(),
      remove_sponsor_segments: removeSponsorSegments(),
      live_from_start: liveFromStart(),
      verbose_logs: verboseLogs(),
      cookie_browser: cookieBrowser(),
      network_stack: networkStack(),
      rate_limit: rateLimit(),
      retry_count: retryCount(),
      fragment_retry_count: fragmentRetryCount(),
      sleep_requests: sleepRequests(),
    };
  }

  function updateActiveJob(patch: Partial<QueueItem>) {
    const id = activeJobId();
    if (id) {
      updateJob(id, patch);
    }
  }

  function updateJob(id: string, patch: Partial<QueueItem>) {
    setQueue((items) => items.map((item) => (item.id === id ? { ...item, ...patch } : item)));
  }

  function appendLog(text: string, stream?: string) {
    setLogs((current) => {
      const next = [...current, { id: crypto.randomUUID(), text, stream }];
      return next.slice(-260);
    });
  }

  return (
    <main class="app-shell">
      <aside class="side-panel">
        <div class="brand">
          <div class="brand-mark" aria-hidden="true">
            <img src="/assets/daedalus.png" alt="" />
          </div>
          <div>
            <h1>Daedalus</h1>
            <p>RELIQUARY</p>
          </div>
        </div>

        <section class="queue-panel" aria-label="Queue">
          <div class="section-title">
            <ListVideo size={16} />
            <span>Queue</span>
          </div>

          <Show when={queue().length > 0} fallback={<p class="empty-state">No jobs yet.</p>}>
            <div class="queue-list">
              <For each={queue()}>
                {(item) => (
                  <article class={`queue-item ${item.status}`}>
                    <div>
                      <strong>{item.title}</strong>
                      <span>{item.status}</span>
                    </div>
                    <div class="progress-track" aria-label={`${item.progress}%`}>
                      <span style={{ width: `${item.progress}%` }} />
                    </div>
                  </article>
                )}
              </For>
            </div>
          </Show>
        </section>

        <div class="side-actions">
          <button
            class={`settings-button ${settingsOpen() ? "active" : ""}`}
            type="button"
            onClick={() => setSettingsOpen(true)}
          >
            <Settings size={17} />
            <span>Settings</span>
          </button>
        </div>
      </aside>

      <section class="workspace" style={{ "grid-template-rows": `auto auto minmax(0, 1fr) ${consoleHeight()}px` }}>
        <header class="topbar">
          <div>
            <p class="eyebrow">Downloader</p>
            <h2>Capture, convert, archive.</h2>
          </div>
          <div class="topbar-actions">
            <span class={`readiness-pill ${toolsReady() ? "ready" : "missing"}`}>
              {toolsReady() ? <CheckCircle2 size={15} /> : <AlertCircle size={15} />}
              <span>{toolsReady() ? "Ready" : "Setup needed"}</span>
            </span>
            <button class="primary-button" type="button" onClick={downloadAll} disabled={!canDownload()}>
              <Download size={17} />
              <span>{isPreparingQueue() ? "Preparing" : isDownloading() ? "Running" : "Download"}</span>
            </button>
          </div>
        </header>

        <div class="message-slot">
          <Show
            when={error()}
            fallback={
              <Show when={!toolsReady()}>
                <div class="setup-strip">
                  <Sparkles size={16} />
                  <span>Install the missing tools once and Daedalus will manage yt-dlp, ffmpeg, and Deno.</span>
                  <button type="button" disabled={Boolean(installingTool())} onClick={() => installTool("all")}>
                    Install
                  </button>
                </div>
              </Show>
            }
          >
            <div class="error-banner" role="alert">
              {error()}
            </div>
          </Show>
        </div>

        <div class="main-grid single-column">
          <section class="input-panel">
            <label class="field-label" for="source-url">
              Source URLs
            </label>
            <textarea
              id="source-url"
              class="url-input"
              spellcheck={false}
              value={urlInput()}
              onInput={(event) => setUrlInput(event.currentTarget.value)}
              placeholder="https://www.youtube.com/watch?v=..."
            />

            <div class="field-grid">
              <label class="text-field">
                <span>
                  <FolderInput size={15} />
                  Output directory
                </span>
                <div class="path-control">
                  <input value={outputDir()} onInput={(event) => setOutputDir(event.currentTarget.value)} />
                  <button type="button" title="Browse output directory" onClick={browseOutputDirectory}>
                    <FolderOpen size={16} />
                  </button>
                </div>
              </label>

              <label class="text-field">
                <span>
                  <Activity size={15} />
                  Fragments
                </span>
                <input
                  type="number"
                  min="1"
                  max="16"
                  value={concurrentFragments()}
                  onInput={(event) => setConcurrentFragments(Number(event.currentTarget.value))}
                />
              </label>
            </div>

            <div class="mode-switch" role="tablist" aria-label="Download mode">
              <button class={mode() === "video" ? "active" : ""} type="button" onClick={() => setMode("video")}>
                <Film size={16} />
                <span>Video</span>
              </button>
              <button class={mode() === "audio" ? "active" : ""} type="button" onClick={() => setMode("audio")}>
                <Music2 size={16} />
                <span>Audio</span>
              </button>
            </div>

            <div class="option-grid">
              <label class="select-field">
                <span>Quality</span>
                <select value={quality()} onChange={(event) => setQuality(event.currentTarget.value)}>
                  <For each={qualityOptions}>{(option) => <option value={option.value}>{option.label}</option>}</For>
                </select>
              </label>

              <Switch>
                <Match when={mode() === "video"}>
                  <label class="select-field">
                    <span>Container</span>
                    <select value={videoFormat()} onChange={(event) => setVideoFormat(event.currentTarget.value)}>
                      <For each={videoFormats}>{(format) => <option value={format}>{format.toUpperCase()}</option>}</For>
                    </select>
                  </label>
                </Match>
                <Match when={mode() === "audio"}>
                  <label class="select-field">
                    <span>Format</span>
                    <select value={audioFormat()} onChange={(event) => setAudioFormat(event.currentTarget.value)}>
                      <For each={audioFormats}>{(format) => <option value={format}>{format.toUpperCase()}</option>}</For>
                    </select>
                  </label>
                </Match>
              </Switch>
            </div>

            <div class="toggles">
              <Toggle label="Playlist" checked={includePlaylist()} onChange={setIncludePlaylist} />
              <Toggle label="Metadata" checked={embedMetadata()} onChange={setEmbedMetadata} />
              <Toggle label="Thumbnail" checked={embedThumbnail()} onChange={setEmbedThumbnail} />
              <Toggle label="Subtitles" checked={writeSubtitles()} onChange={setWriteSubtitles} />
              <Toggle label="Chapters" checked={embedChapters()} onChange={setEmbedChapters} />
              <Toggle label="Skip duplicates" checked={avoidRedownload()} onChange={setAvoidRedownload} />
            </div>
          </section>
        </div>

        <section class="console-panel" aria-label="Console">
          <div class="console-resizer" role="separator" aria-orientation="horizontal" onPointerDown={startConsoleResize} />
          <div class="section-title">
            <Terminal size={16} />
            <span>Console</span>
          </div>
          <div class="console-output" ref={consoleOutputRef}>
            <Show when={logs().length > 0} fallback={<p>Waiting for activity.</p>}>
              <For each={logs()}>
                {(line) => <code class={line.stream === "stderr" ? "stderr" : ""}>{line.text}</code>}
              </For>
            </Show>
          </div>
        </section>
      </section>

      <Show when={settingsOpen()}>
        <div class="settings-overlay" role="presentation" onClick={() => setSettingsOpen(false)}>
        <section class="settings-dialog settings-page" role="dialog" aria-modal="true" aria-label="Daedalus settings" onClick={(event) => event.stopPropagation()}>
          <header class="topbar">
            <div>
              <p class="eyebrow">Daedalus</p>
              <h2>Settings</h2>
            </div>
            <button class="secondary-button" type="button" onClick={() => setSettingsOpen(false)}>
              <X size={17} />
              <span>Close</span>
            </button>
          </header>

          <div class="settings-layout">
            <nav class="settings-nav" aria-label="Settings categories">
              <SettingsNavButton
                icon={<SlidersHorizontal size={16} />}
                label="General"
                active={settingsCategory() === "general"}
                onClick={() => setSettingsCategory("general")}
              />
              <SettingsNavButton
                icon={<Download size={16} />}
                label="Downloads"
                active={settingsCategory() === "downloads"}
                onClick={() => setSettingsCategory("downloads")}
              />
              <SettingsNavButton
                icon={<ListVideo size={16} />}
                label="Playlists"
                active={settingsCategory() === "playlists"}
                onClick={() => setSettingsCategory("playlists")}
              />
              <SettingsNavButton
                icon={<FolderInput size={16} />}
                label="Files"
                active={settingsCategory() === "files"}
                onClick={() => setSettingsCategory("files")}
              />
              <SettingsNavButton
                icon={<Wrench size={16} />}
                label="System tools"
                active={settingsCategory() === "tools"}
                onClick={() => setSettingsCategory("tools")}
              />
            </nav>

            <main class="settings-content">
              <Switch>
                <Match when={settingsCategory() === "general"}>
                  <section class="settings-section">
                    <div class="section-title">
                      <SlidersHorizontal size={16} />
                      <span>General</span>
                    </div>
                    <SettingRow
                      title="Appearance"
                      description="Choose the local Daedalus theme. Each RELIQUARY app now stores its own preference."
                    >
                      <div class="theme-switch" role="group" aria-label="Theme">
                        <button class={themeMode() === "light" ? "active" : ""} type="button" onClick={() => void changeThemeMode("light")}>
                          Light
                        </button>
                        <button class={themeMode() === "dark" ? "active" : ""} type="button" onClick={() => void changeThemeMode("dark")}>
                          Dark
                        </button>
                      </div>
                    </SettingRow>
                    <SettingRow
                      title="App folder"
                      description="Open the local Daedalus folder used for settings and download archives."
                    >
                      <button class="secondary-button" type="button" onClick={openAppFolder}>
                        <FolderOpen size={16} />
                        <span>Open app folder</span>
                      </button>
                    </SettingRow>
                    <SettingRow
                      title="Output directory"
                      description="Default folder used for new downloads. The browse button opens the native folder picker."
                    >
                      <div class="path-control">
                        <input value={outputDir()} onInput={(event) => setOutputDir(event.currentTarget.value)} />
                        <button type="button" title="Browse output directory" onClick={browseOutputDirectory}>
                          <FolderOpen size={16} />
                        </button>
                      </div>
                    </SettingRow>
                  </section>
                </Match>

                <Match when={settingsCategory() === "downloads"}>
                  <section class="settings-section">
                    <div class="section-title">
                      <Download size={16} />
                      <span>Downloads</span>
                    </div>
                    <SettingRow
                      title="Filename schema"
                      description="Controls yt-dlp output names. Add tags to build a reusable naming pattern."
                    >
                      <div class="template-control">
                        <div class="template-input">
                          <Tags size={15} />
                          <input
                            value={filenameTemplate()}
                            spellcheck={false}
                            onInput={(event) => setFilenameTemplate(event.currentTarget.value)}
                          />
                        </div>
                        <div class="tag-list" aria-label="Filename tags">
                          <For each={filenameTags}>
                            {(item) => (
                              <button type="button" onClick={() => insertFilenameTag(item.tag)}>
                                {item.label}
                              </button>
                            )}
                          </For>
                        </div>
                      </div>
                    </SettingRow>
                    <SettingRow
                      title="Skip previously archived downloads"
                      description="Uses yt-dlp's download archive to skip media IDs already marked as completed. Keep this off when you want to freely delete and redownload files."
                    >
                      <SwitchControl checked={avoidRedownload()} onChange={setAvoidRedownload} />
                    </SettingRow>
                    <SettingRow
                      title="Clear download archive"
                      description="Forgets remembered media IDs so deleted files can be downloaded again."
                    >
                      <button class="secondary-button" type="button" disabled={clearingArchive()} onClick={clearArchive}>
                        {clearingArchive() ? "Clearing" : "Clear archive"}
                      </button>
                    </SettingRow>
                    <SettingRow
                      title="Concurrent fragments"
                      description="Number of fragments yt-dlp can download in parallel for segmented sources."
                    >
                      <label class="range-field compact">
                        <input
                          type="range"
                          min="1"
                          max="16"
                          value={concurrentFragments()}
                          onInput={(event) => setConcurrentFragments(Number(event.currentTarget.value))}
                        />
                        <strong>{concurrentFragments()}</strong>
                      </label>
                    </SettingRow>
                    <SettingRow
                      title="Retries"
                      description="How many times yt-dlp retries a failed download before giving up."
                    >
                      <label class="range-field compact">
                        <input
                          type="range"
                          min="0"
                          max="30"
                          value={retryCount()}
                          onInput={(event) => setRetryCount(Number(event.currentTarget.value))}
                        />
                        <strong>{retryCount()}</strong>
                      </label>
                    </SettingRow>
                    <SettingRow
                      title="Fragment retries"
                      description="How many times yt-dlp retries an individual media fragment on segmented streams."
                    >
                      <label class="range-field compact">
                        <input
                          type="range"
                          min="0"
                          max="30"
                          value={fragmentRetryCount()}
                          onInput={(event) => setFragmentRetryCount(Number(event.currentTarget.value))}
                        />
                        <strong>{fragmentRetryCount()}</strong>
                      </label>
                    </SettingRow>
                    <SettingRow
                      title="Rate limit"
                      description="Caps download speed when you want Daedalus to stay gentle on the connection."
                    >
                      <label class="select-field settings-select">
                        <select value={rateLimit()} onChange={(event) => setRateLimit(event.currentTarget.value)}>
                          <For each={rateLimitOptions}>
                            {(option) => <option value={option.value}>{option.label}</option>}
                          </For>
                        </select>
                      </label>
                    </SettingRow>
                    <SettingRow
                      title="Browser cookies"
                      description="Use cookies from a browser for age-gated, private, or account-bound videos."
                    >
                      <label class="select-field settings-select">
                        <select
                          value={cookieBrowser()}
                          onChange={(event) => setCookieBrowser(event.currentTarget.value as (typeof cookieBrowsers)[number])}
                        >
                          <For each={cookieBrowsers}>
                            {(browser) => <option value={browser}>{browser === "none" ? "None" : browser}</option>}
                          </For>
                        </select>
                      </label>
                    </SettingRow>
                    <SettingRow
                      title="Network stack"
                      description="Force IPv4 or IPv6 when a site or network behaves better with one of them."
                    >
                      <label class="select-field settings-select">
                        <select
                          value={networkStack()}
                          onChange={(event) => setNetworkStack(event.currentTarget.value as NetworkStack)}
                        >
                          <option value="auto">Auto</option>
                          <option value="ipv4">Force IPv4</option>
                          <option value="ipv6">Force IPv6</option>
                        </select>
                      </label>
                    </SettingRow>
                    <SettingRow
                      title="Wait between requests"
                      description="Adds a small pause between yt-dlp requests for stricter sites or fragile networks."
                    >
                      <label class="range-field compact">
                        <input
                          type="range"
                          min="0"
                          max="10"
                          value={sleepRequests()}
                          onInput={(event) => setSleepRequests(Number(event.currentTarget.value))}
                        />
                        <strong>{sleepRequests()}s</strong>
                      </label>
                    </SettingRow>
                    <SettingRow
                      title="Embed chapters"
                      description="Write available source chapters into the final video container."
                    >
                      <SwitchControl checked={embedChapters()} onChange={setEmbedChapters} />
                    </SettingRow>
                    <SettingRow
                      title="Save description"
                      description="Writes the source description next to the media file when available."
                    >
                      <SwitchControl checked={writeDescription()} onChange={setWriteDescription} />
                    </SettingRow>
                    <SettingRow
                      title="Save thumbnail file"
                      description="Keeps a separate thumbnail image file instead of only embedding it into the media."
                    >
                      <SwitchControl checked={writeThumbnailFile()} onChange={setWriteThumbnailFile} />
                    </SettingRow>
                    <SettingRow
                      title="Save comments"
                      description="Asks yt-dlp to collect comments and store them in the metadata JSON."
                    >
                      <SwitchControl checked={writeComments()} onChange={setWriteComments} />
                    </SettingRow>
                    <SettingRow
                      title="Playlist metadata files"
                      description="Writes playlist-level metadata files when downloading a playlist."
                    >
                      <SwitchControl checked={writePlaylistMetadata()} onChange={setWritePlaylistMetadata} />
                    </SettingRow>
                    <SettingRow
                      title="Mark as watched"
                      description="Marks supported videos as watched on the source account after download."
                    >
                      <SwitchControl checked={markWatched()} onChange={setMarkWatched} />
                    </SettingRow>
                    <SettingRow
                      title="Remove SponsorBlock segments"
                      description="Cuts supported sponsor, intro, outro, and self-promo segments from the final media."
                    >
                      <SwitchControl checked={removeSponsorSegments()} onChange={setRemoveSponsorSegments} />
                    </SettingRow>
                    <SettingRow
                      title="Live from start"
                      description="For supported live streams, tries downloading from the beginning instead of only the current point."
                    >
                      <SwitchControl checked={liveFromStart()} onChange={setLiveFromStart} />
                    </SettingRow>
                    <SettingRow
                      title="Verbose logs"
                      description="Adds detailed yt-dlp diagnostics to the console when you need to understand a failure."
                    >
                      <SwitchControl checked={verboseLogs()} onChange={setVerboseLogs} />
                    </SettingRow>
                    <SettingRow
                      title="Completion notification"
                      description="Show a system notification after the full queue has finished."
                    >
                      <SwitchControl checked={notifyOnComplete()} onChange={setNotifyOnComplete} />
                    </SettingRow>
                  </section>
                </Match>

                <Match when={settingsCategory() === "playlists"}>
                  <section class="settings-section">
                    <div class="section-title">
                      <ListVideo size={16} />
                      <span>Playlists</span>
                    </div>
                    <SettingRow
                      title="Skip unavailable videos"
                      description="Continue a playlist when one item is private, removed, blocked, or otherwise unavailable."
                    >
                      <SwitchControl checked={skipUnavailable()} onChange={setSkipUnavailable} />
                    </SettingRow>
                    <SettingRow
                      title="Ignore errors"
                      description="Ask yt-dlp to keep going even after broader extraction or download errors."
                    >
                      <SwitchControl checked={ignoreErrors()} onChange={setIgnoreErrors} />
                    </SettingRow>
                  </section>
                </Match>

                <Match when={settingsCategory() === "files"}>
                  <section class="settings-section">
                    <div class="section-title">
                      <FolderInput size={16} />
                      <span>Files</span>
                    </div>
                    <SettingRow
                      title="Restrict filenames"
                      description="Use conservative ASCII filenames. Recommended on Windows to avoid invalid filename errors."
                    >
                      <SwitchControl checked={restrictFilenames()} onChange={setRestrictFilenames} />
                    </SettingRow>
                    <SettingRow
                      title="Prefer free formats"
                      description="Prefer open formats such as WebM/Opus when yt-dlp has several equivalent choices."
                    >
                      <SwitchControl checked={preferFreeFormats()} onChange={setPreferFreeFormats} />
                    </SettingRow>
                    <SettingRow
                      title="Write info JSON"
                      description="Save yt-dlp metadata next to the media file for archival or later automation."
                    >
                      <SwitchControl checked={writeInfoJson()} onChange={setWriteInfoJson} />
                    </SettingRow>
                    <SettingRow
                      title="Keep intermediate files"
                      description="Keep source streams after merging or extracting, useful for debugging conversions."
                    >
                      <SwitchControl checked={keepIntermediate()} onChange={setKeepIntermediate} />
                    </SettingRow>
                    <SettingRow
                      title="Skip certificate checks"
                      description="Bypass TLS certificate validation for broken endpoints. Leave off unless a source requires it."
                    >
                      <SwitchControl checked={noCheckCertificate()} onChange={setNoCheckCertificate} />
                    </SettingRow>
                  </section>
                </Match>

                <Match when={settingsCategory() === "tools"}>
                  <section class="settings-section">
                    <div class="section-title">
                      <Wrench size={16} />
                      <span>System tools</span>
                    </div>
                    <SettingRow
                      title="System tool location"
                      description="Daedalus uses yt-dlp, ffmpeg, and Deno from PATH, winget, or Homebrew."
                    >
                      <p class="settings-note">{systemStatus()?.tools_dir ?? "System tools unavailable."}</p>
                    </SettingRow>
                    <SettingRow
                      title="Open tool location"
                      description="Open the installed system tool folder on disk."
                    >
                      <button class="secondary-button" type="button" onClick={openToolchainFolder}>
                        <FolderOpen size={16} />
                        <span>Open tool location</span>
                      </button>
                    </SettingRow>
                    <ToolRow
                      tool={systemStatus()?.yt_dlp}
                      fallback="yt-dlp"
                      installing={installingTool() === "yt-dlp" || installingTool() === "all"}
                      onInstall={() => installTool("yt-dlp")}
                    />
                    <ToolRow
                      tool={systemStatus()?.ffmpeg}
                      fallback="ffmpeg"
                      installing={installingTool() === "ffmpeg" || installingTool() === "all"}
                      onInstall={() => installTool("ffmpeg")}
                    />
                    <ToolRow
                      tool={systemStatus()?.deno}
                      fallback="deno"
                      installing={installingTool() === "deno" || installingTool() === "all"}
                      onInstall={() => installTool("deno")}
                    />
                    <Show when={missingTools().length > 1}>
                      <button class="install-all-button" type="button" disabled={Boolean(installingTool())} onClick={() => installTool("all")}>
                        <PackageCheck size={15} />
                        <span>{installingTool() === "all" ? "Installing" : "Install missing system tools"}</span>
                      </button>
                    </Show>
                  </section>
                </Match>
              </Switch>
            </main>
          </div>
        </section>
        </div>
      </Show>
    </main>
  );
}

function ToolCard(props: { tool?: ToolStatus; fallback: string; installing: boolean; onInstall: () => void }) {
  const installed = () => props.tool?.installed ?? false;

  return (
    <article class={`tool-card ${installed() ? "ready" : "missing"}`} title={props.tool?.path ?? props.tool?.error ?? props.fallback}>
      <StatusDot installed={installed()} />
      <div>
        <strong>{props.tool?.name ?? props.fallback}</strong>
        <span>{installed() ? toolLocationLabel(props.tool) : "missing"}</span>
      </div>
      <button type="button" disabled={installed() || props.installing} onClick={props.onInstall}>
        {installed() ? <CheckCircle2 size={15} /> : <Download size={15} />}
        <span>{props.installing ? "Installing" : installed() ? "Ready" : "Install"}</span>
      </button>
    </article>
  );
}

function ToolRow(props: { tool?: ToolStatus; fallback: string; installing: boolean; onInstall: () => void }) {
  const installed = () => props.tool?.installed ?? false;

  return (
    <article class={`tool-row ${installed() ? "ready" : "missing"}`} title={props.tool?.path ?? props.tool?.error ?? props.fallback}>
      <div>
          <StatusDot installed={installed()} />
        <div>
          <strong>{props.tool?.name ?? props.fallback}</strong>
          <span>{installed() ? toolDetailLabel(props.tool) : props.tool?.error ?? "Not installed"}</span>
        </div>
      </div>
      <button type="button" disabled={props.installing} onClick={props.onInstall}>
        {props.installing ? "Installing" : installed() ? "Update" : "Install"}
      </button>
    </article>
  );
}

function StatusDot(props: { installed: boolean }) {
  return <span class={`status-dot ${props.installed ? "ready" : "missing"}`} />;
}

function toolLocationLabel(tool?: ToolStatus) {
  const source = tool?.managed ? "package" : "system";
  return tool?.path ? `${source} - ${compactPath(tool.path)}` : source;
}

function toolDetailLabel(tool?: ToolStatus) {
  return tool?.version ? `${tool.version} - ${compactPath(tool.path)}` : toolLocationLabel(tool);
}

function compactPath(path?: string) {
  if (!path) {
    return "";
  }

  return path.split(/[\\/]/).slice(-4).join("\\");
}

function Toggle(props: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  return (
    <button
      class={`toggle ${props.checked ? "checked" : ""}`}
      type="button"
      role="switch"
      aria-checked={props.checked}
      onClick={() => props.onChange(!props.checked)}
    >
      <span class="switch-track" aria-hidden="true">
        <span />
      </span>
      <span>{props.label}</span>
    </button>
  );
}

function SwitchControl(props: { checked: boolean; onChange: (checked: boolean) => void }) {
  return (
    <button
      class={`switch-control ${props.checked ? "checked" : ""}`}
      type="button"
      role="switch"
      aria-checked={props.checked}
      onClick={() => props.onChange(!props.checked)}
    >
      <span />
    </button>
  );
}

function SettingsNavButton(props: { icon: JSX.Element; label: string; active: boolean; onClick: () => void }) {
  return (
    <button class={props.active ? "active" : ""} type="button" onClick={props.onClick}>
      {props.icon}
      <span>{props.label}</span>
    </button>
  );
}

function SettingRow(props: { title: string; description: string; children: JSX.Element }) {
  return (
    <div class="setting-row">
      <div>
        <strong>{props.title}</strong>
        <p>{props.description}</p>
      </div>
      <div class="setting-control">{props.children}</div>
    </div>
  );
}

function labelFromUrl(url: string) {
  try {
    return new URL(url).hostname;
  } catch {
    return url;
  }
}

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function applyThemeMode(themeMode: ThemeMode) {
  document.documentElement.dataset.theme = themeMode;
}

function clampNumber(value: number, min: number, max: number, fallback: number) {
  if (!Number.isFinite(value)) {
    return fallback;
  }

  return Math.max(min, Math.min(max, value));
}

export default App;
