#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let invoked_as_mcp = std::env::args_os()
        .next()
        .and_then(|arg| {
            std::path::PathBuf::from(arg)
                .file_name()
                .map(|name| name.to_owned())
        })
        .is_some_and(|name| name == "issue-tracker-mcp" || name == "issue-tracker-mcp.exe");
    let mcp_subcommand = std::env::args_os().nth(1).is_some_and(|arg| arg == "mcp");
    if invoked_as_mcp || mcp_subcommand {
        if let Err(error) = issue_tracker_lib::run_mcp() {
            eprintln!("Issue Tracker MCP: {error}");
            std::process::exit(1);
        }
    } else {
        issue_tracker_lib::run();
    }
}
