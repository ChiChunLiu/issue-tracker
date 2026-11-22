# Issue Tracker

A local desktop app for collecting issue reports. The frontend uses React and TypeScript, with Tauri and SQLite for local storage. Each report belongs to the active project and has a title, source, description, note, platform, reported time, system info, app version, application log, and tags. Source can be Slack, Reddit, Mail, or Zendesk.

Select a project from the sidebar, or use the **+** beside it to create, rename, and manage projects. Reports and tag suggestions are scoped to the selected project. Existing reports are migrated into **Default project**. An empty project can be deleted; move or delete its reports first if it contains any.

Use the chevron by the app icon to collapse or expand the sidebar. Opening the app again focuses the existing window instead of starting a second instance. The sidebar and operating system icons are generated from the same Nord mark with `python3 scripts/generate-icon.py`.

Click the **Reported time** column heading to switch between newest and oldest first. Reports are paginated at ten per page. The list can also be filtered by each field. Title, description, note, system info, app version, and application log filters accept approximate text matches; source, platform, tag, and time filters use exact values or ranges.

Tags appear as rectangular chips. Type a tag and press Enter or use **Add tag**; existing tags appear as autocomplete suggestions. The temporary **Add 10 sample reports** button inserts ten saved reports, all tagged `sample`, to exercise the list and filters. Each click adds another ten.

**Settings** lets you switch between Nord light and dark themes, choose English or Traditional Chinese, and choose a folder for automatic backups. The selections are saved locally. When a backup folder is set, the app writes `issue-tracker-backup.db` there after saved report, project, or settings changes. The settings page shows the last sync result and has a **Sync now** button.

## Run

Requires [Tauri 2 desktop prerequisites](https://v2.tauri.app/start/prerequisites/), Rust, and Node.js.

```sh
npm install
npm run tauri dev
```

On macOS, build the `.app` with `npm run build`. It appears under `src-tauri/target/release/bundle/macos/`. No DMG is created. `npm run build:frontend` only prepares the web assets.

For another operating system, run the Tauri build with that system's bundle target (for example, `npm run tauri build -- --bundles nsis` on Windows or `--bundles deb` on Linux).

## Data and backups

The app creates `reports.db` under Tauri's app data directory for the identifier `com.ccl.issue-tracker`. The database is local to the device; the app does not send reports or logs to a server.

Open **Settings** from the bottom of the sidebar to use **Export backup** for a consistent SQLite copy, or set an automatic backup folder there. To restore one, close the app, move the existing app data directory aside, create a new app data directory, and place the backup there as `reports.db`. Restart the app. Keeping the old directory aside gives you a way to undo the restore.

The database is not encrypted by the app. Use device encryption if the logs are sensitive.

## Scope

This release supports creating, viewing, editing, and deleting reports. It does not yet group reports into occurrences or issues, deduplicate submissions, track fix state, or ingest external sources.
