use rusqlite::{params, Connection, OptionalExtension, MAIN_DB};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri::{Manager, State};
use uuid::Uuid;

mod ai_import;
mod mcp;

pub fn run_mcp() -> Result<(), String> {
    mcp::run()
}

fn show_main_window(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(true);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "macos")]
fn install_mcp_launcher() -> std::io::Result<()> {
    use std::os::unix::fs::symlink;

    let executable = std::env::current_exe()?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME is not set"))?;
    let bin_dir = home.join(".local/bin");
    let launcher = bin_dir.join("issue-tracker-mcp");
    std::fs::create_dir_all(&bin_dir)?;

    match std::fs::symlink_metadata(&launcher) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            if std::fs::read_link(&launcher)? == executable {
                return Ok(());
            }
            std::fs::remove_file(&launcher)?;
        }
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("{} exists and is not a symlink", launcher.display()),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    symlink(executable, launcher)
}

struct AppState {
    connection: Mutex<Connection>,
    database_path: PathBuf,
    backup_status: Mutex<BackupStatus>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct BackupStatus {
    last_synced_at: String,
    last_error: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
struct Project {
    id: i64,
    name: String,
    report_count: i64,
    issue_count: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AppSettings {
    theme: String,
    language: String,
    backup_folder: String,
    selected_project_id: i64,
    ai_provider: String,
    ai_endpoint: String,
    ai_model: String,
    anthropic_endpoint: String,
    anthropic_model: String,
}

#[derive(Debug, Deserialize)]
struct ReportInput {
    project_id: i64,
    title: String,
    source: String,
    description: String,
    note: String,
    platform: String,
    system_info: String,
    app_version: String,
    application_log: String,
    reported_at: String,
    tags: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ReportFilters {
    project_id: i64,
    title: String,
    source: String,
    description: String,
    note: String,
    platform: String,
    system_info: String,
    app_version: String,
    application_log: String,
    tag: String,
    reported_from: String,
    reported_to: String,
    oldest_first: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ReportSummary {
    id: i64,
    public_id: String,
    project_id: i64,
    title: String,
    platform: String,
    app_version: String,
    reported_at: String,
    tags: Vec<String>,
    issue_id: Option<i64>,
    issue_public_id: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct Report {
    id: i64,
    public_id: String,
    project_id: i64,
    title: String,
    source: String,
    description: String,
    note: String,
    platform: String,
    system_info: String,
    app_version: String,
    application_log: String,
    reported_at: String,
    tags: Vec<String>,
    issue_id: Option<i64>,
    issue_public_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct IssueInput {
    project_id: i64,
    title: String,
    description: String,
    root_cause: String,
    fix: String,
    state: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct Issue {
    id: i64,
    public_id: String,
    project_id: i64,
    title: String,
    description: String,
    root_cause: String,
    fix: String,
    state: String,
    report_count: i64,
    created_at: String,
    updated_at: String,
}

fn open_database(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version == 8 {
        return Ok(connection);
    }
    if !(0..=7).contains(&version) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if version > 0 && path != std::path::Path::new(":memory:") {
        let backup = path.with_file_name(format!("reports-before-uuid-{}.db", Uuid::new_v4()));
        connection.backup(MAIN_DB, &backup, None::<fn(rusqlite::backup::Progress)>)?;
    }
    connection.execute_batch("PRAGMA foreign_keys = OFF;")?;
    connection.execute_batch(
        "BEGIN IMMEDIATE;
         DROP TABLE IF EXISTS report_tags;
         DROP TABLE IF EXISTS reports;
         DROP TABLE IF EXISTS issues;
         DROP TABLE IF EXISTS projects;
         DROP TABLE IF EXISTS app_settings;
         CREATE TABLE projects (
           id INTEGER PRIMARY KEY,
           name TEXT NOT NULL COLLATE NOCASE UNIQUE,
           created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         );
         INSERT INTO projects (id, name) VALUES (1, 'Default project');
         CREATE TABLE issues (
           id INTEGER PRIMARY KEY,
           public_id TEXT NOT NULL UNIQUE,
           project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
           title TEXT NOT NULL,
           description TEXT NOT NULL DEFAULT '',
           root_cause TEXT NOT NULL DEFAULT '',
           fix TEXT NOT NULL DEFAULT '',
           state TEXT NOT NULL CHECK(state IN ('investigating', 'pr-opened', 'pr-merged', 'released')),
           created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
           updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         );
         CREATE INDEX issues_project_updated ON issues(project_id, updated_at DESC);
         CREATE TABLE reports (
           id INTEGER PRIMARY KEY,
           public_id TEXT NOT NULL UNIQUE,
           project_id INTEGER NOT NULL REFERENCES projects(id),
           issue_id INTEGER REFERENCES issues(id) ON DELETE SET NULL,
           title TEXT NOT NULL,
           source TEXT NOT NULL DEFAULT '',
           description TEXT NOT NULL DEFAULT '',
           note TEXT NOT NULL DEFAULT '',
           platform TEXT NOT NULL,
           system_info TEXT NOT NULL DEFAULT '',
           app_version TEXT NOT NULL DEFAULT '',
           application_log TEXT NOT NULL DEFAULT '',
           reported_at TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
           updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         );
         CREATE INDEX reports_project_time ON reports(project_id, reported_at);
         CREATE INDEX reports_issue_id ON reports(issue_id);
         CREATE TABLE report_tags (
           report_id INTEGER NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
           tag TEXT NOT NULL COLLATE NOCASE,
           PRIMARY KEY (report_id, tag)
         );
         CREATE INDEX report_tags_tag ON report_tags(tag);
         CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO app_settings (key, value) VALUES
           ('theme', 'light'), ('language', 'en'), ('backup_folder', ''),
           ('selected_project_id', '1'),
           ('ai_endpoint', 'https://api.together.ai/v1/chat/completions'),
           ('ai_model', 'zai-org/GLM-5.3'),
           ('ai_provider', 'openai-compatible'),
           ('anthropic_endpoint', 'https://api.anthropic.com/v1/messages'),
           ('anthropic_model', 'claude-sonnet-4-6');
         CREATE TRIGGER report_issue_project_insert BEFORE INSERT ON reports
         WHEN NEW.issue_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM issues WHERE id = NEW.issue_id AND project_id = NEW.project_id
         ) BEGIN SELECT RAISE(ABORT, 'Issue and report must belong to the same project'); END;
         CREATE TRIGGER report_issue_project_update BEFORE UPDATE OF issue_id, project_id ON reports
         WHEN NEW.issue_id IS NOT NULL AND NOT EXISTS (
           SELECT 1 FROM issues WHERE id = NEW.issue_id AND project_id = NEW.project_id
         ) BEGIN SELECT RAISE(ABORT, 'Issue and report must belong to the same project'); END;
         PRAGMA user_version = 8;
         COMMIT;",
    )?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(connection)
}

fn normalize_tags(tags: &[String]) -> Result<Vec<String>, String> {
    if tags.len() > 30 {
        return Err("A report can have at most 30 tags.".to_owned());
    }
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > 50 {
            return Err("Each tag must be 50 characters or fewer.".to_owned());
        }
        if !normalized
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(tag))
        {
            normalized.push(tag.to_owned());
        }
    }
    Ok(normalized)
}

fn validate(input: &ReportInput) -> Result<Vec<String>, String> {
    if input.title.trim().is_empty() {
        return Err("Title is required.".to_owned());
    }
    if input.title.chars().count() > 500 {
        return Err("Title must be 500 characters or fewer.".to_owned());
    }
    if !matches!(input.platform.as_str(), "Linux" | "macOS" | "Windows") {
        return Err("Choose Linux, macOS, or Windows.".to_owned());
    }
    if !matches!(
        input.source.as_str(),
        "" | "Slack" | "Reddit" | "Mail" | "Zendesk"
    ) {
        return Err("Choose a valid source.".to_owned());
    }
    if !is_utc_timestamp(&input.reported_at) {
        return Err("Reported time must be a UTC timestamp.".to_owned());
    }
    normalize_tags(&input.tags)
}

fn is_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 24
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'.'
        && bytes[23] == b'Z'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, byte)| matches!(i, 4 | 7 | 10 | 13 | 16 | 19 | 23) || byte.is_ascii_digit())
}

fn read_tags(connection: &Connection, id: i64) -> rusqlite::Result<Vec<String>> {
    let mut query = connection
        .prepare("SELECT tag FROM report_tags WHERE report_id = ?1 ORDER BY tag COLLATE NOCASE")?;
    let tags = query
        .query_map([id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tags)
}

fn read_report(connection: &Connection, id: i64) -> rusqlite::Result<Option<Report>> {
    let report = connection
        .query_row(
            "SELECT id, project_id, title, platform, system_info, app_version, application_log,
                    reported_at, created_at, updated_at, source, description, note, issue_id,
                    public_id, (SELECT public_id FROM issues WHERE id = reports.issue_id)
             FROM reports WHERE id = ?1",
            [id],
            |row| {
                Ok(Report {
                    id: row.get(0)?,
                    public_id: row.get(14)?,
                    project_id: row.get(1)?,
                    title: row.get(2)?,
                    source: row.get(10)?,
                    description: row.get(11)?,
                    note: row.get(12)?,
                    platform: row.get(3)?,
                    system_info: row.get(4)?,
                    app_version: row.get(5)?,
                    application_log: row.get(6)?,
                    reported_at: row.get(7)?,
                    tags: Vec::new(),
                    issue_id: row.get(13)?,
                    issue_public_id: row.get(15)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .optional()?;
    report
        .map(|mut report| {
            report.tags = read_tags(connection, id)?;
            Ok(report)
        })
        .transpose()
}

fn write_tags(connection: &Connection, id: i64, tags: &[String]) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM report_tags WHERE report_id = ?1", [id])?;
    for tag in tags {
        connection.execute(
            "INSERT INTO report_tags (report_id, tag) VALUES (?1, ?2)",
            params![id, tag],
        )?;
    }
    Ok(())
}

fn insert_report(connection: &mut Connection, input: ReportInput) -> Result<Report, String> {
    let tags = validate(&input)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO reports
               (public_id, project_id, title, platform, system_info, app_version, application_log, reported_at,
                source, description, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                Uuid::new_v4().to_string(),
                input.project_id,
                input.title.trim(),
                input.platform,
                input.system_info,
                input.app_version,
                input.application_log,
                input.reported_at,
                input.source,
                input.description,
                input.note
            ],
        )
        .map_err(|error| error.to_string())?;
    let id = transaction.last_insert_rowid();
    write_tags(&transaction, id, &tags).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    read_report(connection, id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "The new report could not be loaded.".to_owned())
}

fn change_report(
    connection: &mut Connection,
    id: i64,
    input: ReportInput,
) -> Result<Report, String> {
    let tags = validate(&input)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let changed = transaction
        .execute(
            "UPDATE reports SET project_id = ?1, title = ?2, platform = ?3, system_info = ?4,
              app_version = ?5, application_log = ?6, reported_at = ?7,
              source = ?8, description = ?9, note = ?10,
              updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?11",
            params![
                input.project_id,
                input.title.trim(),
                input.platform,
                input.system_info,
                input.app_version,
                input.application_log,
                input.reported_at,
                input.source,
                input.description,
                input.note,
                id
            ],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Report not found.".to_owned());
    }
    write_tags(&transaction, id, &tags).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    read_report(connection, id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Report not found.".to_owned())
}

fn insert_sample_reports(connection: &mut Connection, project_id: i64) -> rusqlite::Result<usize> {
    let samples: [(&str, &str, &str, &str, &str, i64, &[&str]); 10] = [
        (
            "Window goes blank after sleep",
            "Windows",
            "Windows 11 · Intel Core i7",
            "2.8.2",
            "[resume] renderer surface unavailable",
            1,
            &["sample", "display"],
        ),
        (
            "App freezes when monitor disconnects",
            "macOS",
            "macOS 15.4 · Apple M3",
            "2.8.2",
            "[display] external monitor removed\n[renderer] heartbeat timeout",
            3,
            &["sample", "freeze", "display"],
        ),
        (
            "Tray icon missing after login",
            "Linux",
            "Ubuntu 24.04 · AMD Ryzen 7",
            "2.8.1",
            "[tray] registration timed out",
            6,
            &["sample", "tray"],
        ),
        (
            "Update download stops at 90 percent",
            "Windows",
            "Windows 11 ARM · Snapdragon X",
            "2.8.1",
            "[updater] checksum mismatch",
            10,
            &["sample", "update"],
        ),
        (
            "Keyboard shortcut stops responding",
            "macOS",
            "macOS 15.3 · Apple M2",
            "2.8.0",
            "[shortcut] accelerator registration failed",
            20,
            &["sample", "keyboard"],
        ),
        (
            "High CPU usage while idle",
            "Linux",
            "Fedora 42 · Intel Core i5",
            "2.8.2",
            "[sync] polling loop exceeded expected frequency",
            30,
            &["sample", "performance"],
        ),
        (
            "Cannot paste into editor",
            "Windows",
            "Windows 10 · AMD Ryzen 5",
            "2.7.9",
            "[clipboard] read denied",
            40,
            &["sample", "editor"],
        ),
        (
            "Settings reset after restart",
            "macOS",
            "macOS 14.7 · Apple M1",
            "2.8.0",
            "[settings] failed to flush preferences",
            52,
            &["sample", "settings"],
        ),
        (
            "Notification appears twice",
            "Linux",
            "Debian 13 · Intel Core i7",
            "2.8.2",
            "[notifications] duplicate listener attached",
            72,
            &["sample", "notifications"],
        ),
        (
            "App crashes while opening a large file",
            "Windows",
            "Windows 11 · Intel Core i9",
            "2.8.2",
            "[file] allocation failed: 512 MB",
            96,
            &["sample", "crash", "files"],
        ),
    ];
    let transaction = connection.transaction()?;
    for (title, platform, system_info, app_version, application_log, hours_ago, tags) in samples {
        let modifier = format!("-{hours_ago} hours");
        transaction.execute(
            "INSERT INTO reports
               (public_id, project_id, title, platform, system_info, app_version, application_log, reported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?8))",
            params![
                Uuid::new_v4().to_string(),
                project_id,
                title,
                platform,
                system_info,
                app_version,
                application_log,
                modifier
            ],
        )?;
        let id = transaction.last_insert_rowid();
        for tag in tags {
            transaction.execute(
                "INSERT INTO report_tags (report_id, tag) VALUES (?1, ?2)",
                params![id, tag],
            )?;
        }
    }
    transaction.commit()?;
    Ok(samples.len())
}

fn all_tags(connection: &Connection, project_id: i64) -> rusqlite::Result<Vec<String>> {
    let mut query = connection.prepare(
        "SELECT DISTINCT t.tag FROM report_tags t JOIN reports r ON r.id = t.report_id
         WHERE r.project_id = ?1 ORDER BY t.tag COLLATE NOCASE",
    )?;
    let tags = query
        .query_map([project_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tags)
}

fn tags_by_report(connection: &Connection) -> rusqlite::Result<HashMap<i64, Vec<String>>> {
    let mut tags = HashMap::<i64, Vec<String>>::new();
    let mut query =
        connection.prepare("SELECT report_id, tag FROM report_tags ORDER BY tag COLLATE NOCASE")?;
    let rows = query.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (id, tag) = row?;
        tags.entry(id).or_default().push(tag);
    }
    Ok(tags)
}

fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    let haystack = haystack.to_lowercase();
    if haystack.contains(&needle) {
        return true;
    }
    let words: Vec<&str> = haystack
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    needle.split_whitespace().all(|term| {
        let allowed = if term.chars().count() >= 7 {
            2
        } else if term.chars().count() >= 3 {
            1
        } else {
            0
        };
        words
            .iter()
            .any(|word| edit_distance_within(word, term, allowed))
    })
}

fn edit_distance_within(a: &str, b: &str, maximum: usize) -> bool {
    if a == b {
        return true;
    }
    if maximum == 0 {
        return false;
    }
    let left: Vec<char> = a.chars().collect();
    let right: Vec<char> = b.chars().collect();
    if left.len() == right.len() {
        let different: Vec<usize> = left
            .iter()
            .zip(&right)
            .enumerate()
            .filter_map(|(index, (a, b))| (a != b).then_some(index))
            .collect();
        if different.len() == 2
            && different[1] == different[0] + 1
            && left[different[0]] == right[different[1]]
            && left[different[1]] == right[different[0]]
        {
            return true;
        }
    }
    if left.len().abs_diff(right.len()) > maximum || left.len() > 100 || right.len() > 100 {
        return false;
    }
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];
    for (i, char_a) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, char_b) in right.iter().enumerate() {
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + usize::from(char_a != char_b));
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()] <= maximum
}

fn query_reports(
    connection: &Connection,
    filters: &ReportFilters,
) -> rusqlite::Result<Vec<ReportSummary>> {
    let tags = tags_by_report(connection)?;
    let mut query = connection.prepare(
        "SELECT id, project_id, title, platform, system_info, app_version,
                CASE WHEN ?1 = '' THEN '' ELSE application_log END, reported_at,
                source, description, note, issue_id, public_id,
                (SELECT public_id FROM issues WHERE id = reports.issue_id)
         FROM reports ORDER BY reported_at DESC, id DESC",
    )?;
    let rows = query.query_map([&filters.application_log], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, Option<i64>>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, Option<String>>(13)?,
        ))
    })?;
    let mut reports = Vec::new();
    for row in rows {
        let (
            id,
            project_id,
            title,
            platform,
            system_info,
            app_version,
            application_log,
            reported_at,
            source,
            description,
            note,
            issue_id,
            public_id,
            issue_public_id,
        ) = row?;
        let report_tags = tags.get(&id).cloned().unwrap_or_default();
        if filters.project_id != 0 && filters.project_id != project_id
            || !filters.platform.is_empty() && filters.platform != platform
            || !filters.reported_from.is_empty() && reported_at < filters.reported_from
            || !filters.reported_to.is_empty() && reported_at > filters.reported_to
            || !filters.tag.is_empty()
                && !report_tags
                    .iter()
                    .any(|tag| tag.eq_ignore_ascii_case(&filters.tag))
            || !fuzzy_match(&title, &filters.title)
            || !fuzzy_match(&system_info, &filters.system_info)
            || !fuzzy_match(&app_version, &filters.app_version)
            || !fuzzy_match(&application_log, &filters.application_log)
            || !filters.source.is_empty() && source != filters.source
            || !fuzzy_match(&description, &filters.description)
            || !fuzzy_match(&note, &filters.note)
        {
            continue;
        }
        reports.push(ReportSummary {
            id,
            public_id,
            project_id,
            title,
            platform,
            app_version,
            reported_at,
            tags: report_tags,
            issue_id,
            issue_public_id,
        });
    }
    if filters.oldest_first {
        reports.reverse();
    }
    Ok(reports)
}

fn issue_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Issue> {
    Ok(Issue {
        id: row.get(0)?,
        public_id: row.get(10)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        root_cause: row.get(4)?,
        fix: row.get(5)?,
        state: row.get(6)?,
        report_count: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const ISSUE_SELECT: &str = "SELECT i.id, i.project_id, i.title, i.description, i.root_cause,
    i.fix, i.state, COUNT(r.id), i.created_at, i.updated_at, i.public_id
    FROM issues i LEFT JOIN reports r ON r.issue_id = i.id";

fn read_issue(connection: &Connection, id: i64) -> rusqlite::Result<Option<Issue>> {
    connection
        .query_row(
            &format!("{ISSUE_SELECT} WHERE i.id = ?1 GROUP BY i.id"),
            [id],
            issue_from_row,
        )
        .optional()
}

fn issues_for_project(connection: &Connection, project_id: i64) -> rusqlite::Result<Vec<Issue>> {
    let mut query = connection.prepare(&format!(
        "{ISSUE_SELECT} WHERE i.project_id = ?1 GROUP BY i.id ORDER BY i.updated_at DESC, i.id DESC"
    ))?;
    let rows = query.query_map([project_id], issue_from_row)?;
    rows.collect()
}

fn validate_issue(input: &IssueInput) -> Result<(), String> {
    if input.title.trim().is_empty() || input.title.chars().count() > 500 {
        return Err("Issue title must be 1–500 characters.".to_owned());
    }
    if !matches!(
        input.state.as_str(),
        "investigating" | "pr-opened" | "pr-merged" | "released"
    ) {
        return Err("Select a valid issue state.".to_owned());
    }
    if input.description.chars().count() > 100_000
        || input.root_cause.chars().count() > 100_000
        || input.fix.chars().count() > 100_000
    {
        return Err("Issue text is too long.".to_owned());
    }
    Ok(())
}

fn insert_issue(connection: &Connection, input: IssueInput) -> Result<Issue, String> {
    validate_issue(&input)?;
    connection
        .execute(
            "INSERT INTO issues (public_id, project_id, title, description, root_cause, fix, state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                Uuid::new_v4().to_string(),
                input.project_id,
                input.title.trim(),
                input.description,
                input.root_cause,
                input.fix,
                input.state
            ],
        )
        .map_err(|error| error.to_string())?;
    read_issue(connection, connection.last_insert_rowid())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "The new issue could not be loaded.".to_owned())
}

fn change_issue(connection: &Connection, id: i64, input: IssueInput) -> Result<Issue, String> {
    validate_issue(&input)?;
    let changed = connection
        .execute(
            "UPDATE issues SET title = ?1, description = ?2, root_cause = ?3, fix = ?4,
         state = ?5, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?6 AND project_id = ?7",
            params![
                input.title.trim(),
                input.description,
                input.root_cause,
                input.fix,
                input.state,
                id,
                input.project_id
            ],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Issue not found in this project.".to_owned());
    }
    read_issue(connection, id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Issue not found.".to_owned())
}

fn set_report_issue(
    connection: &Connection,
    report_id: i64,
    issue_id: Option<i64>,
) -> Result<Report, String> {
    let report = read_report(connection, report_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Report not found.".to_owned())?;
    if let Some(issue_id) = issue_id {
        let issue = read_issue(connection, issue_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Issue not found.".to_owned())?;
        if issue.project_id != report.project_id {
            return Err("Issue and report must belong to the same project.".to_owned());
        }
    }
    connection.execute(
        "UPDATE reports SET issue_id = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?2",
        params![issue_id, report_id],
    ).map_err(|error| error.to_string())?;
    read_report(connection, report_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Report not found.".to_owned())
}

fn read_setting(connection: &Connection, key: &str) -> rusqlite::Result<String> {
    connection.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
}

fn write_setting(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn read_settings(connection: &Connection) -> rusqlite::Result<AppSettings> {
    Ok(AppSettings {
        theme: read_setting(connection, "theme")?,
        language: read_setting(connection, "language")?,
        backup_folder: read_setting(connection, "backup_folder")?,
        selected_project_id: read_setting(connection, "selected_project_id")?
            .parse()
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        ai_provider: read_setting(connection, "ai_provider")?,
        ai_endpoint: read_setting(connection, "ai_endpoint")?,
        ai_model: read_setting(connection, "ai_model")?,
        anthropic_endpoint: read_setting(connection, "anthropic_endpoint")?,
        anthropic_model: read_setting(connection, "anthropic_model")?,
    })
}

fn project_exists(connection: &Connection, id: i64) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1)",
        [id],
        |row| row.get(0),
    )
}

fn read_project(connection: &Connection, id: i64) -> rusqlite::Result<Option<Project>> {
    connection
        .query_row(
            "SELECT p.id, p.name, COUNT(r.id), (SELECT COUNT(*) FROM issues i WHERE i.project_id = p.id) FROM projects p
         LEFT JOIN reports r ON r.project_id = p.id WHERE p.id = ?1 GROUP BY p.id",
            [id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    report_count: row.get(2)?,
                    issue_count: row.get(3)?,
                })
            },
        )
        .optional()
}

fn projects(connection: &Connection) -> rusqlite::Result<Vec<Project>> {
    let mut query = connection.prepare(
        "SELECT p.id, p.name, COUNT(r.id), (SELECT COUNT(*) FROM issues i WHERE i.project_id = p.id) FROM projects p
         LEFT JOIN reports r ON r.project_id = p.id
         GROUP BY p.id ORDER BY p.created_at, p.id",
    )?;
    let rows = query.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            report_count: row.get(2)?,
            issue_count: row.get(3)?,
        })
    })?;
    rows.collect()
}

fn validate_project_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err("Project name must be 1–100 characters.".to_owned());
    }
    Ok(name)
}

fn create_project_record(connection: &Connection, name: &str) -> Result<Project, String> {
    let name = validate_project_name(name)?;
    connection
        .execute("INSERT INTO projects (name) VALUES (?1)", [name])
        .map_err(|error| error.to_string())?;
    read_project(connection, connection.last_insert_rowid())
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Project not found after creation.".to_owned())
}

fn rename_project_record(connection: &Connection, id: i64, name: &str) -> Result<Project, String> {
    let name = validate_project_name(name)?;
    let changed = connection
        .execute(
            "UPDATE projects SET name = ?1 WHERE id = ?2",
            params![name, id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Project not found.".to_owned());
    }
    read_project(connection, id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Project not found.".to_owned())
}

fn delete_project_record(connection: &Connection, id: i64) -> Result<(), String> {
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if count <= 1 {
        return Err("Keep at least one project.".to_owned());
    }
    let project = read_project(connection, id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Project not found.".to_owned())?;
    if project.report_count > 0 || project.issue_count > 0 {
        return Err("Delete the project's reports and issues before deleting it.".to_owned());
    }
    connection
        .execute("DELETE FROM projects WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    let settings = read_settings(connection).map_err(|error| error.to_string())?;
    if settings.selected_project_id == id {
        let replacement: i64 = connection
            .query_row("SELECT id FROM projects ORDER BY id LIMIT 1", [], |row| {
                row.get(0)
            })
            .map_err(|error| error.to_string())?;
        write_setting(connection, "selected_project_id", &replacement.to_string())
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn sync_backup_if_configured(state: &AppState, connection: &Connection) -> Result<(), String> {
    let folder = read_setting(connection, "backup_folder").map_err(|error| error.to_string())?;
    if folder.is_empty() {
        return Ok(());
    }
    let destination = PathBuf::from(folder).join("issue-tracker-backup.db");
    let outcome = connection
        .backup(
            MAIN_DB,
            &destination,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(|error| error.to_string());
    let mut status = state
        .backup_status
        .lock()
        .map_err(|error| error.to_string())?;
    match outcome {
        Ok(()) => {
            status.last_synced_at = connection
                .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
                    row.get(0)
                })
                .map_err(|error| error.to_string())?;
            status.last_error.clear();
            Ok(())
        }
        Err(error) => {
            status.last_error = error.clone();
            Err(error)
        }
    }
}

#[tauri::command]
fn list_reports(
    filters: ReportFilters,
    state: State<'_, AppState>,
) -> Result<Vec<ReportSummary>, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    query_reports(&connection, &filters).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_issues(project_id: i64, state: State<'_, AppState>) -> Result<Vec<Issue>, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    issues_for_project(&connection, project_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_issue(id: i64, state: State<'_, AppState>) -> Result<Issue, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    read_issue(&connection, id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Issue not found.".to_owned())
}

#[tauri::command]
fn create_issue(input: IssueInput, state: State<'_, AppState>) -> Result<Issue, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let issue = insert_issue(&connection, input)?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(issue)
}

#[tauri::command]
fn update_issue(id: i64, input: IssueInput, state: State<'_, AppState>) -> Result<Issue, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let issue = change_issue(&connection, id, input)?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(issue)
}

#[tauri::command]
fn delete_issue(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let changed = connection
        .execute("DELETE FROM issues WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Issue not found.".to_owned());
    }
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(())
}

#[tauri::command]
fn link_report_issue(
    report_id: i64,
    issue_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Report, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let report = set_report_issue(&connection, report_id, issue_id)?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(report)
}

#[tauri::command]
fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    projects(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
fn create_project(name: String, state: State<'_, AppState>) -> Result<Project, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let project = create_project_record(&connection, &name)?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(project)
}

#[tauri::command]
fn rename_project(id: i64, name: String, state: State<'_, AppState>) -> Result<Project, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let project = rename_project_record(&connection, id, &name)?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(project)
}

#[tauri::command]
fn delete_project(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    delete_project_record(&connection, id)?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(())
}

#[tauri::command]
fn select_project(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    if !project_exists(&connection, id).map_err(|error| error.to_string())? {
        return Err("Project not found.".to_owned());
    }
    write_setting(&connection, "selected_project_id", &id.to_string())
        .map_err(|error| error.to_string())?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    read_settings(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
fn update_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    if !matches!(settings.theme.as_str(), "light" | "dark")
        || !matches!(settings.language.as_str(), "en" | "zh-TW")
    {
        return Err("Invalid theme or language.".to_owned());
    }
    if !settings.backup_folder.is_empty() && !PathBuf::from(&settings.backup_folder).is_dir() {
        return Err("The backup folder is unavailable.".to_owned());
    }
    ai_import::validate_ai_config(&settings)?;
    let mut connection = state.connection.lock().map_err(|error| error.to_string())?;
    if !project_exists(&connection, settings.selected_project_id)
        .map_err(|error| error.to_string())?
    {
        return Err("Project not found.".to_owned());
    }
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    write_setting(&transaction, "theme", &settings.theme).map_err(|error| error.to_string())?;
    write_setting(&transaction, "language", &settings.language)
        .map_err(|error| error.to_string())?;
    write_setting(&transaction, "backup_folder", &settings.backup_folder)
        .map_err(|error| error.to_string())?;
    write_setting(&transaction, "ai_endpoint", settings.ai_endpoint.trim())
        .map_err(|error| error.to_string())?;
    write_setting(&transaction, "ai_model", settings.ai_model.trim())
        .map_err(|error| error.to_string())?;
    write_setting(&transaction, "ai_provider", &settings.ai_provider)
        .map_err(|error| error.to_string())?;
    write_setting(
        &transaction,
        "anthropic_endpoint",
        settings.anthropic_endpoint.trim(),
    )
    .map_err(|error| error.to_string())?;
    write_setting(
        &transaction,
        "anthropic_model",
        settings.anthropic_model.trim(),
    )
    .map_err(|error| error.to_string())?;
    write_setting(
        &transaction,
        "selected_project_id",
        &settings.selected_project_id.to_string(),
    )
    .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    let _ = sync_backup_if_configured(&state, &connection);
    read_settings(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
fn get_backup_status(state: State<'_, AppState>) -> Result<BackupStatus, String> {
    state
        .backup_status
        .lock()
        .map(|status| status.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_backup_now(state: State<'_, AppState>) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let folder = read_setting(&connection, "backup_folder").map_err(|error| error.to_string())?;
    if folder.is_empty() {
        return Err("Choose a backup folder first.".to_owned());
    }
    sync_backup_if_configured(&state, &connection)
}

#[tauri::command]
fn list_tags(project_id: i64, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    all_tags(&connection, project_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn seed_sample_reports(project_id: i64, state: State<'_, AppState>) -> Result<usize, String> {
    if !cfg!(debug_assertions) {
        return Err("Sample reports are only available in development builds.".to_owned());
    }
    let mut connection = state.connection.lock().map_err(|error| error.to_string())?;
    let count =
        insert_sample_reports(&mut connection, project_id).map_err(|error| error.to_string())?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(count)
}

#[tauri::command]
fn get_report(id: i64, state: State<'_, AppState>) -> Result<Report, String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    read_report(&connection, id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Report not found.".to_owned())
}

#[tauri::command]
fn create_report(input: ReportInput, state: State<'_, AppState>) -> Result<Report, String> {
    let mut connection = state.connection.lock().map_err(|error| error.to_string())?;
    let report = insert_report(&mut connection, input)?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(report)
}

#[tauri::command]
fn update_report(
    id: i64,
    input: ReportInput,
    state: State<'_, AppState>,
) -> Result<Report, String> {
    let mut connection = state.connection.lock().map_err(|error| error.to_string())?;
    let report = change_report(&mut connection, id, input)?;
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(report)
}

#[tauri::command]
fn delete_report(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    let changed = connection
        .execute("DELETE FROM reports WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Report not found.".to_owned());
    }
    let _ = sync_backup_if_configured(&state, &connection);
    Ok(())
}

#[tauri::command]
fn backup_database(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let destination = PathBuf::from(path);
    if destination.as_os_str().is_empty() || destination == state.database_path {
        return Err("Choose a different backup file.".to_owned());
    }
    let connection = state.connection.lock().map_err(|error| error.to_string())?;
    connection
        .backup(
            MAIN_DB,
            &destination,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        show_main_window(app);
    }));
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    let _ = window.app_handle().set_dock_visibility(false);
                }
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            if let Err(error) = install_mcp_launcher() {
                eprintln!("Could not install Issue Tracker MCP launcher: {error}");
            }
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database_path = data_dir.join("reports.db");
            let connection = open_database(&database_path)?;
            app.manage(AppState {
                connection: Mutex::new(connection),
                database_path,
                backup_status: Mutex::new(BackupStatus::default()),
            });
            let show_app = MenuItem::with_id(app, "show-app", "Show app", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_app, &quit])?;
            let mut tray = TrayIconBuilder::new().tooltip("Issue Tracker");
            #[cfg(target_os = "macos")]
            {
                tray = tray
                    .icon(tauri::image::Image::from_bytes(include_bytes!(
                        "../icons/tray.png"
                    ))?)
                    .icon_as_template(true);
            }
            #[cfg(not(target_os = "macos"))]
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show-app" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_reports,
            list_issues,
            get_issue,
            create_issue,
            update_issue,
            delete_issue,
            link_report_issue,
            list_projects,
            create_project,
            rename_project,
            delete_project,
            select_project,
            get_settings,
            update_settings,
            get_backup_status,
            sync_backup_now,
            list_tags,
            seed_sample_reports,
            get_report,
            create_report,
            update_report,
            delete_report,
            backup_database,
            ai_import::has_ai_token,
            ai_import::save_ai_token,
            ai_import::delete_ai_token,
            ai_import::generate_ai_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running issue tracker");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(title: &str) -> ReportInput {
        ReportInput {
            project_id: 1,
            title: title.to_owned(),
            source: "Slack".to_owned(),
            description: "Repro after waking from sleep".to_owned(),
            note: "Needs triage".to_owned(),
            platform: "macOS".to_owned(),
            system_info: "macOS 15.4\nApple M3".to_owned(),
            app_version: "2.8.2".to_owned(),
            application_log: "a log\nwith lines".to_owned(),
            reported_at: "2026-11-22T10:00:00.000Z".to_owned(),
            tags: vec!["Display".to_owned(), "freeze".to_owned()],
        }
    }

    #[test]
    fn reports_round_trip_and_filter() {
        let mut connection = open_database(std::path::Path::new(":memory:")).unwrap();
        let settings = read_settings(&connection).unwrap();
        assert_eq!(
            settings.ai_endpoint,
            "https://api.together.ai/v1/chat/completions"
        );
        assert_eq!(settings.ai_model, "zai-org/GLM-5.3");
        assert_eq!(settings.ai_provider, "openai-compatible");
        assert_eq!(
            settings.anthropic_endpoint,
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(settings.anthropic_model, "claude-sonnet-4-6");
        let created = insert_report(&mut connection, input("Freeze on display")).unwrap();
        assert!(Uuid::parse_str(&created.public_id).is_ok());
        assert_eq!(created.tags, vec!["Display", "freeze"]);
        assert_eq!(created.platform, "macOS");
        assert_eq!(created.source, "Slack");
        assert_eq!(created.description, "Repro after waking from sleep");
        assert_eq!(created.note, "Needs triage");
        let matches = query_reports(
            &connection,
            &ReportFilters {
                title: "freze".to_owned(),
                tag: "display".to_owned(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id, created.id);
        assert_eq!(
            query_reports(
                &connection,
                &ReportFilters {
                    source: "Slack".to_owned(),
                    description: "waking".to_owned(),
                    note: "triage".to_owned(),
                    ..Default::default()
                }
            )
            .unwrap()
            .len(),
            1
        );
        assert!(query_reports(
            &connection,
            &ReportFilters {
                platform: "Linux".to_owned(),
                ..Default::default()
            }
        )
        .unwrap()
        .is_empty());
        assert_eq!(
            query_reports(
                &connection,
                &ReportFilters {
                    application_log: "lgo".to_owned(),
                    system_info: "Aple".to_owned(),
                    ..Default::default()
                }
            )
            .unwrap()
            .len(),
            1
        );
        let mut updated = input("Updated");
        updated.tags = vec!["Crash".to_owned()];
        let updated = change_report(&mut connection, created.id, updated).unwrap();
        assert_eq!(updated.public_id, created.public_id);
        assert_eq!(updated.tags, vec!["Crash"]);
    }

    #[test]
    fn sorting_and_time_filter_use_reported_time() {
        let mut connection = open_database(std::path::Path::new(":memory:")).unwrap();
        let first = insert_report(&mut connection, input("Earlier")).unwrap();
        let mut later = input("Later");
        later.reported_at = "2026-11-23T10:00:00.000Z".to_owned();
        let second = insert_report(&mut connection, later).unwrap();
        let newest = query_reports(&connection, &ReportFilters::default()).unwrap();
        assert_eq!(newest[0].id, second.id);
        let oldest = query_reports(
            &connection,
            &ReportFilters {
                oldest_first: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(oldest[0].id, first.id);
        let filtered = query_reports(
            &connection,
            &ReportFilters {
                reported_from: "2026-11-23T00:00:00.000Z".to_owned(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, second.id);
    }

    #[test]
    fn old_database_is_backed_up_then_reset_for_public_uuids() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("issue-tracker-migration-{unique}"));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("reports.db");
        {
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE reports (
                  id INTEGER PRIMARY KEY, title TEXT NOT NULL, system_info TEXT NOT NULL DEFAULT '',
                  app_version TEXT NOT NULL DEFAULT '', application_log TEXT NOT NULL DEFAULT '',
                  created_at TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 INSERT INTO reports VALUES (1, 'Old report', 'macOS', '2.8.2', 'old log',
                   '2026-11-01T12:00:00.000Z', '2026-11-01T12:00:00.000Z');
                 PRAGMA user_version = 1;",
                )
                .unwrap();
        }
        let connection = open_database(&path).unwrap();
        assert!(query_reports(&connection, &ReportFilters::default())
            .unwrap()
            .is_empty());
        assert!(issues_for_project(&connection, 1).unwrap().is_empty());
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 8);
        let backup = std::fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("reports-before-uuid-")
            })
            .unwrap();
        let original = Connection::open(backup).unwrap();
        let title: String = original
            .query_row("SELECT title FROM reports WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(title, "Old report");
        drop(connection);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn blank_title_and_invalid_platform_are_rejected() {
        let mut connection = open_database(std::path::Path::new(":memory:")).unwrap();
        assert!(insert_report(&mut connection, input("  ")).is_err());
        let mut invalid = input("Report");
        invalid.platform = "Other".to_owned();
        assert!(insert_report(&mut connection, invalid).is_err());
    }

    #[test]
    fn issues_group_reports_within_a_project_and_unlink_on_delete() {
        let mut connection = open_database(std::path::Path::new(":memory:")).unwrap();
        let second_project = create_project_record(&connection, "Second").unwrap();
        let issue_input = IssueInput {
            project_id: 1,
            title: "Wake failure".to_owned(),
            description: "Blank after resume".to_owned(),
            root_cause: "Renderer not restored".to_owned(),
            fix: "Recreate renderer".to_owned(),
            state: "investigating".to_owned(),
        };
        let issue = insert_issue(&connection, issue_input.clone()).unwrap();
        assert_eq!(
            read_project(&connection, 1).unwrap().unwrap().issue_count,
            1
        );
        assert!(delete_project_record(&connection, 1).is_err());
        let first = insert_report(&mut connection, input("First wake report")).unwrap();
        let second = insert_report(&mut connection, input("Second wake report")).unwrap();
        assert_eq!(
            set_report_issue(&connection, first.id, Some(issue.id))
                .unwrap()
                .issue_id,
            Some(issue.id)
        );
        set_report_issue(&connection, second.id, Some(issue.id)).unwrap();
        assert_eq!(
            read_issue(&connection, issue.id)
                .unwrap()
                .unwrap()
                .report_count,
            2
        );

        let mut other_input = input("Other project report");
        other_input.project_id = second_project.id;
        let other = insert_report(&mut connection, other_input).unwrap();
        assert!(set_report_issue(&connection, other.id, Some(issue.id)).is_err());
        let mut invalid_move = input("Moved report");
        invalid_move.project_id = second_project.id;
        assert!(change_report(&mut connection, first.id, invalid_move).is_err());
        assert_eq!(
            read_report(&connection, first.id)
                .unwrap()
                .unwrap()
                .project_id,
            1
        );

        let mut merged = issue_input;
        merged.state = "pr-merged".to_owned();
        assert_eq!(
            change_issue(&connection, issue.id, merged).unwrap().state,
            "pr-merged"
        );
        connection
            .execute("DELETE FROM issues WHERE id = ?1", [issue.id])
            .unwrap();
        assert_eq!(
            read_report(&connection, first.id)
                .unwrap()
                .unwrap()
                .issue_id,
            None
        );
        assert_eq!(
            read_report(&connection, second.id)
                .unwrap()
                .unwrap()
                .issue_id,
            None
        );
    }

    #[test]
    fn backup_keeps_tags_and_reported_time() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("issue-tracker-backup-{unique}"));
        std::fs::create_dir(&directory).unwrap();
        let backup_path = directory.join("backup.db");
        let mut connection = open_database(std::path::Path::new(":memory:")).unwrap();
        let created = insert_report(&mut connection, input("Backed up")).unwrap();
        connection
            .backup(
                MAIN_DB,
                &backup_path,
                None::<fn(rusqlite::backup::Progress)>,
            )
            .unwrap();
        let backup = open_database(&backup_path).unwrap();
        let restored = read_report(&backup, created.id).unwrap().unwrap();
        assert_eq!(restored.tags, vec!["Display", "freeze"]);
        assert_eq!(restored.reported_at, created.reported_at);
        drop(backup);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn sample_button_adds_ten_filterable_reports() {
        let mut connection = open_database(std::path::Path::new(":memory:")).unwrap();
        assert_eq!(insert_sample_reports(&mut connection, 1).unwrap(), 10);
        let reports = query_reports(
            &connection,
            &ReportFilters {
                tag: "sample".to_owned(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reports.len(), 10);
        assert!(reports.iter().any(|report| report.platform == "Linux"));
        assert!(reports.iter().any(|report| report.platform == "macOS"));
        assert!(reports.iter().any(|report| report.platform == "Windows"));
    }

    #[test]
    fn projects_scope_reports_tags_and_synced_backup() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("issue-tracker-projects-{unique}"));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("reports.db");
        let mut connection = open_database(&path).unwrap();
        let second = create_project_record(&connection, "Second app").unwrap();
        let first_report = insert_report(&mut connection, input("First app issue")).unwrap();
        let mut other_input = input("Second app issue");
        other_input.project_id = second.id;
        other_input.tags = vec!["second-only".to_owned()];
        let second_report = insert_report(&mut connection, other_input).unwrap();
        let first_scope = query_reports(
            &connection,
            &ReportFilters {
                project_id: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let second_scope = query_reports(
            &connection,
            &ReportFilters {
                project_id: second.id,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            first_scope
                .iter()
                .map(|report| report.id)
                .collect::<Vec<_>>(),
            vec![first_report.id]
        );
        assert_eq!(
            second_scope
                .iter()
                .map(|report| report.id)
                .collect::<Vec<_>>(),
            vec![second_report.id]
        );
        assert!(!all_tags(&connection, 1)
            .unwrap()
            .contains(&"second-only".to_owned()));
        assert_eq!(
            read_project(&connection, second.id)
                .unwrap()
                .unwrap()
                .report_count,
            1
        );
        assert!(delete_project_record(&connection, second.id).is_err());

        write_setting(&connection, "theme", "dark").unwrap();
        write_setting(&connection, "language", "zh-TW").unwrap();
        write_setting(&connection, "backup_folder", directory.to_str().unwrap()).unwrap();
        let state = AppState {
            connection: Mutex::new(connection),
            database_path: path,
            backup_status: Mutex::new(BackupStatus::default()),
        };
        sync_backup_if_configured(&state, &state.connection.lock().unwrap()).unwrap();
        assert!(!state
            .backup_status
            .lock()
            .unwrap()
            .last_synced_at
            .is_empty());
        let backup = open_database(&directory.join("issue-tracker-backup.db")).unwrap();
        assert_eq!(read_settings(&backup).unwrap().theme, "dark");
        assert_eq!(read_settings(&backup).unwrap().language, "zh-TW");
        assert_eq!(projects(&backup).unwrap().len(), 2);
        assert_eq!(
            read_report(&backup, second_report.id)
                .unwrap()
                .unwrap()
                .project_id,
            second.id
        );
        drop(backup);
        drop(state);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
