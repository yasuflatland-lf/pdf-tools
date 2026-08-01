use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri should have a repository root")
        .to_path_buf()
}

fn read_repository_file(path: &str) -> String {
    std::fs::read_to_string(repository_root().join(path))
        .unwrap_or_else(|error| panic!("failed to read {path}: {error}"))
}

fn registered_commands() -> BTreeSet<String> {
    let source = read_repository_file("src-tauri/src/lib.rs");
    let marker = "tauri::generate_handler![";
    let body = source
        .split_once(marker)
        .expect("lib.rs should contain tauri::generate_handler!")
        .1
        .split_once(']')
        .expect("generate_handler! should have a closing bracket")
        .0;

    body.split(',')
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(str::to_owned)
        .collect()
}

fn invoked_commands() -> BTreeSet<String> {
    let source = read_repository_file("src/lib/tauri-api.ts");
    let mut commands = BTreeSet::new();
    let mut remaining = source.as_str();

    while let Some(offset) = remaining.find("invoke") {
        remaining = &remaining[offset + "invoke".len()..];
        let Some(open_paren) = remaining.find('(') else {
            break;
        };
        let arguments = remaining[open_paren + 1..].trim_start();
        let Some(arguments) = arguments.strip_prefix('"') else {
            continue;
        };
        let Some(close_quote) = arguments.find('"') else {
            break;
        };
        commands.insert(arguments[..close_quote].to_owned());
        remaining = &arguments[close_quote + 1..];
    }

    commands
}

fn documented_commands() -> BTreeSet<String> {
    let source = read_repository_file("docs/architecture.md");
    let section = source
        .split_once("## Command surface and IPC")
        .expect("architecture.md should contain the command surface section")
        .1;
    let mut commands = BTreeSet::new();
    let mut in_table = false;

    for line in section.lines() {
        if !line.starts_with('|') {
            if in_table {
                break;
            }
            continue;
        }
        in_table = true;
        let cell = line
            .split('|')
            .nth(1)
            .expect("the command table should have a first column");
        let mut remaining = cell;
        while let Some(open_tick) = remaining.find('`') {
            remaining = &remaining[open_tick + 1..];
            let Some(close_tick) = remaining.find('`') else {
                break;
            };
            let signature = &remaining[..close_tick];
            if let Some((name, _)) = signature.split_once('(') {
                commands.insert(name.to_owned());
            }
            remaining = &remaining[close_tick + 1..];
        }
    }

    commands
}

fn commands_pending_frontend_integration() -> BTreeSet<String> {
    // Folder expansion is intentionally registered one issue before the UI
    // begins calling and documenting it.
    BTreeSet::from(["expand_paths".to_owned()])
}

#[test]
fn every_registered_command_has_a_frontend_caller() {
    let registered = registered_commands();
    let invoked = invoked_commands();
    let pending = commands_pending_frontend_integration();
    let missing = registered
        .difference(&invoked)
        .filter(|command| !pending.contains(*command))
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "commands registered in generate_handler! without a frontend invoke: {}",
        missing.join(", ")
    );
}

#[test]
fn documented_commands_match_registered_commands() {
    let pending = commands_pending_frontend_integration();
    let registered = registered_commands()
        .difference(&pending)
        .cloned()
        .collect::<BTreeSet<_>>();
    let documented = documented_commands();
    let missing = registered
        .difference(&documented)
        .cloned()
        .collect::<Vec<_>>();
    let extra = documented
        .difference(&registered)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "command table differs from generate_handler!; missing from docs: {}; extra in docs: {}",
        missing.join(", "),
        extra.join(", ")
    );
}
