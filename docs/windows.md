# Windows

Quest Journal can be built and installed per user on Windows 10/11 x64. Windows CI builds the real MSVC application and native host, runs the Rust tests, and checks desktop and native-host installation on Windows. A successful workflow is required before treating Windows support as verified; Linux checks alone do not establish that the Windows app works.

## Build and install from source

Install these prerequisites:

- Node.js 22 or newer and npm.
- Rust with the `stable-x86_64-pc-windows-msvc` toolchain.
- Microsoft Visual Studio C++ Build Tools, including **Desktop development with C++** and the Windows SDK.
- Microsoft Edge WebView2 Runtime. The per-user installer copies the app executable; it does not install WebView2.

See [Tauri's official Windows prerequisites](https://v2.tauri.app/start/prerequisites/). Run these commands in PowerShell from the repository root:

```powershell
npm ci
npm run install:desktop
npm run setup:browsers
```

The desktop installer copies the executable to `%LOCALAPPDATA%\Quest Journal\bin\quest-journal.exe` and creates a **Quest Journal** Start Menu shortcut. Administrator access is unnecessary. Close Quest Journal before installing an update because Windows locks a running executable, then reopen it from Start. The installer verifies the copied executable matches the build and preserves journal files.

Load `%LOCALAPPDATA%\Quest Journal\browser-extension` using **Load unpacked** on the browser's extensions page with developer mode enabled. The native-host installer copies the host into its permanent per-user directory and registers its manifest under `HKEY_CURRENT_USER`. Use `node scripts/install-native-host.mjs --browser edge` (or `chrome`, `brave`, `chromium`) to select a browser explicitly. The included extension build targets Chromium browsers. Consult the installer's `--help` output for browser limitations, and confirm the extension reports a working native-host connection.

New captures remember which registered browser saved them. Reopening uses that browser's standard installation under Local AppData or Program Files; custom browser installation paths are not yet supported. If that browser cannot be found, the app reports it. Captures without a recognized Windows browser use the system default browser.

Desktop and native host share `%APPDATA%\Quest Journal\Quest Journal\data\quests.sqlite3`, the default journal database selected by Rust's `directories` crate. `QUEST_JOURNAL_DB` can override that path; use the same absolute value for the desktop process and native-host setup when customizing it. The host installer saves that value in a sidecar configuration beside its permanent executable. Set the desktop's override persistently for your user account if you launch it from Start. Updates preserve journal data. To transfer an existing journal, close applications on both systems and copy the SQLite database and any journal/WAL companion files together.

## Validation

```powershell
npm run check
npm run build
node scripts/test-native-install-plan.mjs
cargo test --workspace
npm --workspace @quest-journal/desktop run build:app
cargo build -p quest-native-host --release
node scripts/test-desktop-install-windows.mjs
node scripts/test-native-install.mjs
```

The desktop installation test uses real Windows shortcut COM APIs and Unicode paths, verifies the GUI subsystem in the built executable, checks that a locked executable survives a failed update, verifies safe reinstall and existing journal preservation, and removes its temporary build copy before checking the shortcut. The native installation test sends a framed native-messaging request after removing its temporary build copy. CI does not replace an interactive check of the UI and the browser extension. Windows execution cannot be verified on a Linux development host; review the Windows workflow result and test the installed app on Windows before release.

The Windows workflow uploads the built desktop executable, native host, extension, installation scripts, and this guide. Extract the whole artifact and run:

```powershell
node scripts/install-desktop.mjs --binary quest-desktop.exe
node scripts/install-native-host.mjs --binary quest-native-host.exe --extension browser-extension
```

Artifact installation requires Node.js and WebView2, but does not require Rust or C++ Build Tools. These are unsigned development executables, not a signed MSI or NSIS release installer.
