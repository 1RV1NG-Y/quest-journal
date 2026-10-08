# Quest Journal — Current Product Direction

Updated September 9, 2026. This direction supersedes the earlier phased spec and its main/side and territory requirements. Development proceeds through useful, reviewable increments, rather than an MVP boundary.

## Purpose

Keep the intention and working context of an activity so it can be safely paused and continued. Preserve self-directed exploration; captured resources are not obligations.

## Organization

A quest is an activity or pursuit. Quests may contain smaller quests and materials. Trading can contain Market Structure, a particular course, and Strategy Research. Job Search can contain concept study, LeetCode, data structures, and individual applications.

Main/side designations and territories are retired from everyday interaction. Existing values are retained for compatibility. Nesting describes organization, not importance. Parent quests must never form cycles.

Materials have a current location and manual order independent of historical save points. Initially these are captured browser tabs and referenced files. Moving a material must not silently rewrite old saves. A future migration must define deduplication and preserve existing data.

## Workspace

The resizable left sidebar is the primary quest navigation. Selecting a quest displays its contents in the main pane. Nested quests will expand in the sidebar and appear within their parent. Breadcrumbs will support navigation through nesting.

Use compact material rows and direct manipulation. Implement multiselection, move, cut/paste, undo, and drag-and-drop in subsequent increments. Keep routine navigation and inspection in place. Dialogs can remain for existing save/restore actions until those flows are integrated.

Use neutral dark surfaces, sans-serif typography, subtle separators, and restrained accents. Avoid the card dashboard, large decorative headings, repeated branding, and persistent explanatory panels. A compact integrated header replaces the separate decorated title bar while retaining window controls, drag, resize, and maximize behavior.

## Save semantics

Continue reconstructs useful context, not process memory. Historical saves are distinct from the current material collection. Browser captures save only the captured tabs plus attached files. Desktop Save & pause snapshots the current material collection plus attached files. Neither reconstructs its contents from all historical saves. Continue still restores the latest saved session; opening current materials is a separate direct action.

## Current implementation increment

- Quest navigation replaces territories and main/side groups.
- Quests can be created inside another quest, expanded in the sidebar, and navigated through breadcrumbs.
- An in-place move control relocates a quest and its descendants to another parent or the top level. The storage layer rejects cycles.
- Existing databases migrate automatically; existing quests remain at the top level and saves remain attached to their original quest.
- The workspace displays current browser materials and attached files directly. Materials migrate once from historical captures, deduplicated by exact URL within each quest, retaining the latest captured metadata.
- Select tabs individually, by Shift range, or all at once; open them directly or move them to another quest in place. Moves preserve historical saves and reject destination URL conflicts atomically.
- Material moves append in source display order at the destination. A new capture can intentionally add the same URL to a different quest.
- Existing quest editing, attachment, pause, restore preview, and state changes remain available.
- Window decorations are replaced by an integrated header.

Undo, cut/paste, manual reordering, and drag-and-drop are upcoming work.

## Technical foundation

Retain Tauri, Svelte, Rust, SQLite, and the browser extension/native messaging integration. Preserve user data during migrations. Verify each increment through relevant checks and interaction review.

## Window and navigation refinements

Open maximized. Keep one top-level New quest control at the top of the sidebar. Quest order is manual and persistent among siblings; activity and captures must not change it. Support dragging to reorder and accessible up/down controls. Moving a quest to another parent appends it after that parent's existing quests.

## Open design issue: activity state

The user considers a quest paused whenever it is not being worked on. Current saved state does not track real attention: Save & pause writes a snapshot/checkpoint and a paused flag; Continue restores a saved session and writes an active flag. Retain these controls for now, but reassess their semantics and prominence before adding more state controls.

New captures never remove existing materials. For example, capturing C into a quest containing A and B leaves A, B, and C in the collection. The new browser session contains C, while previous saves remain stored. The current latest-save-only UI needs clearer distinction from the full collection and access to older sessions in a later increment.

## Tab trash

Each browser material has a quick trash action. Bulk selection includes Trash in the Move to destination picker. Trashed tabs are excluded from the current collection, direct opening, and new local snapshots; historical saves are unchanged. An expandable per-quest Trash section restores individual tabs or all tabs, including after restarting. Explicitly capturing a trashed URL again restores that material. File trash icons remove references only, never the actual local files. Native select controls use explicit dark surfaces and readable foreground colors.

## Browser capture workspace

The installer copies the native host to a permanent `bin` directory beside the journal database and points browser launchers to that copy. The installed extension must remain functional when Cargo build outputs are cleaned. Verify the installed host with the native messaging framing protocol, including from the Brave Flatpak runtime.

Unpacked Chromium updates must use a content-addressed service-worker filename and install the manifest after its assets. A browser restart may reuse the old cached worker when only background.js is overwritten. The popup performs at most one automatic extension reload per release when it detects an outdated preview protocol, preserving preferences and saved data. Verify the production manifest's referenced worker, not only the TypeScript source.

The extension uses the desktop's neutral dark design. Its searchable destination picker displays full parent paths and marks nested quests. It remembers the last destination locally and creates quests inline, optionally beneath the selected destination.

One selectable tab list replaces redundant focused-tab and single-tab scopes. This window and All windows scopes cover normal windows in the current browser, grouped by window with window-wide selection, Select all, Clear, and Shift range selection. The popup resolves its own browser window once and passes that ID with every preview and capture request; background focus must never determine “This window.” Refreshing or switching scopes keeps that origin. A closed origin fails explicitly, and a tab moved outside the chosen scope before saving requires a refreshed preview. Initial selection uses the originating window’s current tab group, highlighted tabs, or active tab.

Add tabs updates the material collection without changing state, checkpoint, or latest save. Save session captures selected tabs plus attached files with a checkpoint and keeps tabs open. Save & close closes selected tabs only after persistence succeeds. Tabs that changed since preview require a refresh; tabs that navigate after persistence remain open. Existing materials and historical saves are preserved.

Verification includes scripts/test-extension.mjs for browser API boundary behavior, Rust storage tests, static checks, and a rendered sample-data UI walkthrough. A live Brave/Helium walkthrough remains dependent on browser access. Reload the installed extension or restart the browser after updates.

## Pick something — first increment

A shuffle control beside the sidebar heading opens an in-workspace picker. Choose all quests or the selected quest and its descendants. Independent Top-level quests, Subquests (at any nested depth), and Elements filters can be combined; the default includes both quest types, with elements off. Elements are current browser materials and attached file references, excluding trash and historical-only resources. Quest activity state and sidebar search do not silently filter the pool.

Display the pool size and use equal odds per eligible item. A short vertical name shuffle slows to reveal the selected item's title, full parent context, and type. Reduced-motion preferences and single-item pools reveal directly. Picking has no launch or storage side effects. Open navigates to a quest, launches only the selected tab, or opens only the selected attached file; stale references and launch failures surface an error. Pick again draws independently and may repeat. Empty pools explain how to adjust the filters.

Session skips, avoiding repeats, remembered filters, and balancing by quest belong to a later increment. Verification: `node scripts/test-shuffle.mjs`, desktop static/build checks, and a sample-data interaction walkthrough.

Checkpoint notes (“Where did you leave off?”) are optional in desktop and browser saves. A blank note produces a save with no new note and preserves the quest’s existing checkpoint; editing the quest can explicitly clear it. Tabs, file references, and earlier saves are preserved.

## Quest actions and recoverable deletion

Right-click a quest in the sidebar or a subquest in the contents pane for Rename, New subquest, Move to, and Delete. A sidebar ellipsis exposes the same menu without right-clicking. Menus support arrow keys, Escape, and keyboard context-menu activation. Actions apply to the clicked quest even when another quest is selected.

Delete moves the quest and its currently visible descendants to Quest Trash atomically, preserving tabs, file references, notes, and save history. The immediate Undo action and persistent Quest Trash view restore that deletion group. Previously deleted branches stay independently deleted. Restoring returns to the original parent when available, otherwise to the top level. Deleted quests are absent from navigation, capture destinations, and the shuffle pool; stale saves or moves into them are rejected. Referenced files on disk are never deleted.

## Quest icons

Create and Edit quest include an optional searchable icon picker, with General and Tech categories and a Reset action. The initial offline catalog has 24 general Lucide icons and 28 Devicon logos, including .NET, Java, and C#. The selection is saved per quest, independent of its title or parent, and displayed in navigation, quest headings, nested quest rows, Quest Trash, and quest shuffle results. Existing quests retain the default folder. Missing/unknown identifiers render as the default folder.

Only a stable identifier is stored; icon data and logos ship with the app, with no runtime network requests or user-supplied SVG execution. Devicon SVGs are vendored from @iconify-json/devicon 1.2.68 using scripts/vendor-quest-icons.py, with upstream license and attribution alongside the assets. Renaming, changing state, saving, moving, and restoring a quest preserve its icon. Older clients that omit the icon on updates must not clear it.
