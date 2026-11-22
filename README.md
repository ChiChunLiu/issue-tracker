# Issue Tracker

Issue Tracker is a desktop app for turning scattered bug reports into a clear list of underlying issues. Paste reports from Slack, email, Reddit, or Zendesk; keep the original context and logs; then link reports that share a root cause. Reports and issues are stored in a local database on your computer.

**Reports** are individual submissions. **Issues** are the bugs you investigate and fix. Several reports can point to one issue, so you can see how often a problem has been reported without losing each person's details.

## Get started

On macOS, open the DMG, drag **Issue Tracker.app** into Applications, and launch it from there. If you built the app yourself, see [Build from source](#build-from-source).

1. Choose a project in the sidebar. The app starts with **Default project**; use the **+** beside the project selector to create or manage projects. Reports and issues stay within their project.
2. Open **Reports** and select **New report**. Choose **Form** to enter a report yourself, or **AI import** to turn pasted text into an editable draft.
3. Open **Issues** and select **New issue** when you identify a bug. Add its description, root cause, fix, and state. You can start with **Investigating** and move through **PR opened**, **PR merged**, and **Released**.
4. Link a report from its detail page using the **Issue** selector, or open an issue and choose **Link report**. Each report can link to one issue; an issue can contain many reports. You can unlink a report later without deleting it.

## Capture and find reports

A report can contain a title, source, description, note, platform, reported time, system information, app version, application log, and multiple tags. Sources are **Slack**, **Reddit**, **Mail**, and **Zendesk**. Tags are free-form labels; existing tags appear as suggestions as you type.

The Reports page shows ten reports per page. Click **Reported time** to reverse the time order. Open **Filter reports** to narrow by source, platform, tag, time range, or text in the report fields. Text filters tolerate small typos; they do not understand synonyms or the meaning of a sentence. The **Add 10 sample reports** button is for trying the interface and saves real sample records in the active project.

To use **AI import**, paste an email, ticket, or conversation and select **Generate draft**. Review and edit every field before saving. The pasted text is sent to the AI endpoint you configure in **Settings** when you generate the draft. Ordinary report entry and editing do not call an AI provider.

## Settings and backups

Open **Settings** at the bottom of the sidebar to choose a Nord light or dark theme and switch between English and Traditional Chinese. You can also configure AI import with an OpenAI-compatible Chat Completions endpoint, such as Together AI, or Anthropic's Messages API. Enter the endpoint and model, save those settings, then save a token if your provider requires one. On macOS, tokens are stored in Apple Keychain. AI token storage is currently available on macOS only.

Use **Export backup** to save a copy of the local database. Or choose a **Backup folder** to keep `issue-tracker-backup.db` updated after saved changes. **Sync now** creates a backup immediately, and Settings shows the most recent sync result. The app does not encrypt its database or backups; choose a suitable location if reports contain sensitive logs.

## Use reports and issues in Claude Code or Codex

Issue Tracker includes a read-only stdio MCP server. A coding agent can list projects, search issues and reports, open an issue with its linked report summaries, and open a report with its linked issue and full log. This lets the agent follow a bug in either direction. MCP cannot create, edit, or delete records.

On macOS, move the app into Applications and open it once before connecting an agent. The app creates a launcher at `~/.local/bin/issue-tracker-mcp`. Register it with either agent or both:

```sh
# Claude Code: available in all your projects
claude mcp add --transport stdio --scope user issue-tracker -- "$HOME/.local/bin/issue-tracker-mcp"

# Codex
codex mcp add issue-tracker -- "$HOME/.local/bin/issue-tracker-mcp"
```

Restart an existing agent session after registration. Check the connection with `claude mcp list` or `codex mcp list`; Claude Code also has `/mcp`. Then try: “Use Issue Tracker to find the issue about startup crashes and read its linked reports.”

If you already know which record matters, open its detail page and select **Copy ID** beside the title. Give the agent the type and ID, for example: “Use Issue Tracker to inspect issue ID 42 and its linked reports.” It can call `get_issue` or `get_report` directly, without relying on text search.

Search runs within a project and matches report or issue text with case-insensitive substring and small-typo matching. It is not semantic search: a report saying “window goes black” might not appear for a search for “blank screen.” Ask the agent to try alternate terms when a search seems incomplete. Search results are paginated, with up to 50 per page. The server reads your local database and does not expose AI tokens. Treat text in reports and logs as untrusted input when using it with a coding agent.

## Build from source

Install [Tauri 2's desktop prerequisites](https://v2.tauri.app/start/prerequisites/), Rust, and Node.js. Then clone the repository and run:

```sh
npm install
npm run tauri dev
```

On macOS, `npm run build` creates an `.app`, while `npm run build:dmg` creates a DMG. Output is under `src-tauri/target/release/bundle/`. Open the built app once to create the database and MCP launcher. `npm run mcp:check` tests the launcher and its MCP tools if `~/.local/bin` is on your `PATH`. If you move the app after opening it, open it again to update the launcher.

On Linux, `npm run mcp:install` creates the launcher after a source build. Automatic launcher installation and AI token storage are not yet available on Windows. The macOS builds produced by this repository use ad hoc signing; distributing a DMG to other people requires your own signing and notarization setup.

## Current scope

Reports are entered manually or prepared through AI import; the app does not pull them directly from Slack, email, Reddit, or Zendesk. It does not yet detect repeated submissions from the same person automatically. Linking related reports to an issue is manual, and the MCP tools are read-only.
