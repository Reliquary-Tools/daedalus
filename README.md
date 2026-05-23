# Daedalus

Daedalus is the RELIQUARY video downloader app. It is designed as a Tauri desktop app with a SolidJS interface and a Rust command layer that drives `yt-dlp` and `ffmpeg`.

## Requirements

- Node.js
- Rust
- Optional: `yt-dlp` and `ffmpeg` available in `PATH`

If `yt-dlp` or `ffmpeg` are missing, Daedalus can download managed Windows copies into the user app data directory from the Settings panel.

## Scripts

```powershell
npm install
npm run dev
```

`npm run dev` starts the Tauri desktop app. `npm run build` creates a production desktop bundle.

## Direction

The app is intentionally standalone for now. Shared RELIQUARY behavior such as licensing, update channels, app discovery, and Obelisk integration should later move into shared crates/packages.

## Licence
Copyright (c) 2026 Antoine / Seoloon.  
All rights reserved.

You may not copy, modify, distribute, sublicense, or use this source code, in whole or in part, without explicit written permission.
