# Quest Journal

A local desktop journal for the things you're working on. A **quest** is an activity or pursuit — a course, a job search, a research thread — and it keeps the browser tabs and files that belong to it, so you can put it down and pick it back up later without losing the context.

It's built with Tauri, Svelte, Rust, and SQLite, plus a browser extension that captures tabs into quests through native messaging. Everything stays on your machine.

## What it does

- **Nested quests.** Quests can contain smaller quests (Trading → a course → Strategy Research). Navigate them from the resizable sidebar or breadcrumbs, reorder siblings by dragging, and move a quest with all its descendants to another parent.
- **Materials.** Each quest holds captured browser tabs and attached file references. Select tabs individually, with Shift ranges, or all at once; open them, move them to another quest, or send them to a per-quest Trash you can restore from. Trashing a file only removes the reference, never the file.
- **Save & pause / Continue.** Save a snapshot of a quest's current materials with a checkpoint note, and later restore the latest saved session. Historical saves are kept separate from the current collection and are never rewritten by later moves.
- **Pick something.** A shuffle picker chooses a random quest, subquest, or material (tab or file), from everything or from one quest's subtree, and opens it on request.
- **Browser extension.** From the popup, pick tabs in this window or across all windows, choose a destination quest (searchable, with full parent paths, or created inline), then **Add tabs**, **Save session**, or **Save & close**. Tabs close only after the save succeeds.

## Requirements

- Linux (the installers currently target Linux)
- Node.js 22+ and npm
- Rust (stable) and the [Tauri 2 Linux prerequisites](https://v2.tauri.app/start/prerequisites/)
- A Chromium-based browser for the extension (tested with Helium and Brave)

## Develop

```sh
npm install
npm --workspace @quest-journal/desktop run app   # desktop app with live reload
npm run dev:extension                            # extension dev build (WXT)
```

`npm run check` runs static checks for both apps.

## Install

Desktop app — builds a release binary and installs it to `~/.local/bin/quest-journal` with an application-menu entry:

```sh
npm run install:desktop
```

Browser integration — builds the extension and the native host, copies the host to a permanent location next to the journal database, and registers it with the detected browsers:

```sh
npm run setup:browsers
```

Then load the unpacked extension from `apps/extension/.output/chrome-mv3`. The native host installer detects Helium and Brave (Flatpak) profiles automatically and also supports Chrome, Chromium, and Firefox via `--browser`; run `node scripts/install-native-host.mjs --help` for options. After updating, reload the extension or restart the browser.

## Data

The journal is a single SQLite database, by default in your platform data directory (`~/.local/share/questjournal/quests.sqlite3` on Linux). Set `QUEST_JOURNAL_DB` to use a different file. Migrations run automatically and preserve existing data.

## Project layout

```
apps/desktop           Tauri + Svelte desktop app
apps/extension         WXT + Svelte browser extension
crates/quest-core      Domain types
crates/quest-storage   SQLite storage and migrations
crates/quest-adapters  Validates and reopens saved tabs and files
crates/quest-native-host  Native messaging host used by the extension
scripts/               Installers and regression checks
```

## Tests

```sh
cargo test                              # storage and core tests
node scripts/test-extension.mjs         # extension background behavior
node scripts/test-extension-update.mjs  # extension update handling
node scripts/test-shuffle.mjs           # shuffle picker
node scripts/test-native-install.mjs    # installed native host (after a release build)
```

The current product direction is in [quest_journal_spec.md](quest_journal_spec.md).

## License

MIT
