# Issue Tracker

A local desktop app for collecting issue reports. The frontend uses React and TypeScript, with Tauri and SQLite for local storage. Each report belongs to the active project and has a title, source, description, note, platform, reported time, system info, app version, application log, and tags. Source can be Slack, Reddit, Mail, or Zendesk.

Select a project from the sidebar, or use the **+** beside it to create, rename, and manage projects. Reports and tag suggestions are scoped to the selected project. Existing reports are migrated into **Default project**.

**Issues** sits below Reports in the sidebar. An issue represents one underlying bug within the active project. It has a title, description, root cause, fix, and state: Investigating, PR opened, PR merged, or Released. Several reports can link to one issue, while each report links to at most one issue. Link from a report's detail page or add unlinked reports from an issue's detail page. Deleting an issue leaves its reports intact and unlinked. A project can be deleted only after its reports and issues are removed.

Use the chevron by the app icon to collapse or expand the sidebar. Opening the app again focuses the existing window instead of starting a second instance. The sidebar and operating system icons are generated from the same Nord mark with `python3 scripts/generate-icon.py`.

Click the **Reported time** column heading to switch between newest and oldest first. Reports are paginated at ten per page. The list can also be filtered by each field. Title, description, note, system info, app version, and application log filters accept approximate text matches; source, platform, tag, and time filters use exact values or ranges.

Tags appear as rectangular chips. Type a tag and press Enter or use **Add tag**; existing tags appear as autocomplete suggestions. The temporary **Add 10 sample reports** button inserts ten saved reports, all tagged `sample`, to exercise the list and filters. Each click adds another ten.

**Settings** lets you switch between Nord light and dark themes, choose English or Traditional Chinese, and choose a folder for automatic backups. The selections are saved locally. When a backup folder is set, the app writes `issue-tracker-backup.db` there after saved report, project, or settings changes. The settings page shows the last sync result and has a **Sync now** button.

**New report** offers **Form** and **AI import**. AI import accepts pasted source text, sends it to the configured AI provider, and opens an editable report draft. Review the fields and save it to the active project. The pasted text is held in the current app session; only the reviewed report is saved. **Settings** offers OpenAI-compatible Chat Completions endpoints (including Together AI) and Anthropic's native Messages API. Each provider keeps its own endpoint and model: Together AI with `zai-org/GLM-5.3`, or Anthropic with `claude-sonnet-4-6`. The active provider's token is stored in Apple Keychain on macOS, separately for each provider and endpoint. Anthropic requires a token; local OpenAI-compatible endpoints may work without one. Remote endpoints must use HTTPS; localhost may use HTTP. AI import currently requires macOS for token storage. If you saved a key before the app bundle was signed, enter it once more in Settings; the earlier Keychain item remains under account `together-ai-api-key` and can be removed in Keychain Access.

## Run

Requires [Tauri 2 desktop prerequisites](https://v2.tauri.app/start/prerequisites/), Rust, and Node.js.

```sh
npm install
npm run tauri dev
```

On macOS, build the `.app` with `npm run build`. It appears under `src-tauri/target/release/bundle/macos/`. No DMG is created. `npm run build:frontend` only prepares the web assets.

The local build uses ad hoc signing. For Keychain access that survives replacing the app with a new build, sign successive builds with the same macOS code signing identity. Tauri accepts one through `APPLE_SIGNING_IDENTITY`. Without a stable identity, a rebuilt app may need the AI token entered again.

For another operating system, run the Tauri build with that system's bundle target (for example, `npm run tauri build -- --bundles nsis` on Windows or `--bundles deb` on Linux).

## Data and backups

The app creates `reports.db` under Tauri's app data directory for the identifier `com.ccl.issue-tracker`. The database is local to the device. AI import sends the text you paste to your configured endpoint when you click **Generate draft**; regular report creation and editing do not send report data to an AI endpoint.

Open **Settings** from the bottom of the sidebar to use **Export backup** for a consistent SQLite copy, or set an automatic backup folder there. To restore one, close the app, move the existing app data directory aside, create a new app data directory, and place the backup there as `reports.db`. Restart the app. Keeping the old directory aside gives you a way to undo the restore.

The database is not encrypted by the app. Use device encryption if the logs are sensitive.

## Scope

This release supports creating, viewing, editing, and deleting reports and issues, and linking reports to their underlying issue. It does not yet deduplicate repeated submissions from the same user or ingest external sources automatically. MCP integration is planned for later.
