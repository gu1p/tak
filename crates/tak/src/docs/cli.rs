use clap::{Arg, Command};

#[derive(Debug)]
pub(super) struct CliDocEntry {
    pub(super) path: String,
    pub(super) summary: String,
    pub(super) args: Vec<CliArgDoc>,
}

#[derive(Debug)]
pub(super) struct CliArgDoc {
    pub(super) syntax: String,
    pub(super) summary: String,
}

pub(super) fn collect_cli_docs() -> Vec<CliDocEntry> {
    let mut entries = Vec::new();
    let root = crate::cli::command_tree();
    collect_command_docs(&root, "tak".to_string(), &mut entries);
    entries
}

fn collect_command_docs(command: &Command, path: String, entries: &mut Vec<CliDocEntry>) {
    if path != "tak" {
        let summary = command_doc_summary(command);
        let args = command
            .get_arguments()
            .filter_map(|arg| {
                let summary = arg_doc_summary(arg)?;
                let syntax = render_arg_syntax(&path, arg)?;
                Some(CliArgDoc { syntax, summary })
            })
            .collect::<Vec<_>>();

        entries.push(CliDocEntry {
            path: path.clone(),
            summary,
            args,
        });
    }

    for subcommand in command.get_subcommands() {
        let subcommand_path = format!("{path} {}", subcommand.get_name());
        collect_command_docs(subcommand, subcommand_path, entries);
    }
}

fn command_doc_summary(command: &Command) -> String {
    command
        .get_long_about()
        .or_else(|| command.get_about())
        .map(|text| text.to_string())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn arg_doc_summary(arg: &Arg) -> Option<String> {
    if arg.get_id().as_str() == "help" || arg.get_id().as_str() == "version" || arg.is_hide_set() {
        return None;
    }

    let text = arg
        .get_long_help()
        .or_else(|| arg.get_help())
        .map(|help| help.to_string())?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn render_arg_syntax(command_path: &str, arg: &Arg) -> Option<String> {
    if let Some(long) = arg.get_long() {
        return Some(format!("`{command_path} --{long}`"));
    }
    if let Some(short) = arg.get_short() {
        return Some(format!("`{command_path} -{short}`"));
    }

    let placeholder = arg
        .get_value_names()
        .and_then(|names| names.first().map(|name| name.to_string()))
        .unwrap_or_else(|| arg.get_id().as_str().to_string());
    if placeholder.is_empty() {
        None
    } else {
        Some(format!("`{command_path} <{placeholder}>`"))
    }
}
