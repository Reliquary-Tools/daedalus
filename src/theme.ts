import { invoke } from "@tauri-apps/api/core";

type ThemeMode = "light" | "dark";

function normalizeThemeMode(value: string): ThemeMode {
  return value === "dark" ? "dark" : "light";
}

function applyThemeMode(themeMode: ThemeMode) {
  document.documentElement.dataset.theme = themeMode;
}

export function installReliquaryThemeSync() {
  let active = true;
  let currentTheme: ThemeMode | "" = "";

  const syncTheme = async () => {
    try {
      const nextTheme = normalizeThemeMode(await invoke<string>("get_theme_mode"));
      if (active && nextTheme !== currentTheme) {
        currentTheme = nextTheme;
        applyThemeMode(nextTheme);
      }
    } catch {
      if (active && currentTheme !== "light") {
        currentTheme = "light";
        applyThemeMode("light");
      }
    }
  };

  void syncTheme();
  const interval = window.setInterval(syncTheme, 2500);

  return () => {
    active = false;
    window.clearInterval(interval);
  };
}
