use crate::app::Driver;
use clap::{Arg, ArgAction, Command};
use cli_command_kit::{HelpOptions, Mode, TreeOptions};
use std::path::PathBuf;
pub fn help_options() -> HelpOptions {
    HelpOptions {
        skill: Some(
            cli_command_kit::SkillSource::new("alexgorbatchev/better-fonts-cli", "main")
                .expect("the fixed skill source names a GitHub repository"),
        ),
        tree: TreeOptions {
            hide_generated_commands: true,
            ..Default::default()
        },
        ..Default::default()
    }
}
pub fn command(mode: Mode) -> Command {
    let target = || {
        Arg::new("apps")
            .value_name("APP")
            .num_args(0..)
            .help("App IDs, names or .app paths; omitted uses configured apps")
    };
    let mutation = |name: &'static str, description: &'static str| {
        Command::new(name)
            .about(description)
            .arg(target())
            .arg(
                Arg::new("driver")
                    .long("driver")
                    .value_parser(clap::value_parser!(Driver))
                    .help("Choose electron or native-hook instead of automatic selection"),
            )
            .arg(
                Arg::new("restart")
                    .long("restart")
                    .action(ArgAction::Set)
                    .num_args(0..=1)
                    .require_equals(true)
                    .default_missing_value("true")
                    .value_parser(clap::value_parser!(bool))
                    .conflicts_with("no-restart")
                    .help("Override configured restart behavior"),
            )
            .arg(
                Arg::new("no-restart")
                    .long("no-restart")
                    .action(ArgAction::SetTrue)
                    .help("Leave applications closed after replacing their bundles"),
            )
            .arg(
                Arg::new("dry-run")
                    .long("dry-run")
                    .action(ArgAction::SetTrue)
                    .help("Inspect targets without compiling or changing bundles"),
            )
    };
    let root = Command::new("better-fonts")
        .about("Apply custom fonts to macOS applications with staged bundle replacement")
        .disable_version_flag(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("version")
                .long("version")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print the raw version string"),
        )
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .global(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Use this TOML configuration file"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print detailed per-app operation results"),
        )
        .subcommand(
            Command::new("app")
                .about("Manage application font patches")
                .subcommand_required(true)
                .subcommand(
                    mutation("patch", "Apply the configured font").arg(
                        Arg::new("font")
                            .long("font")
                            .short('f')
                            .value_parser(font_value)
                            .help("Override global and per-app font settings"),
                    ),
                )
                .subcommand(mutation("unpatch", "Restore original application fonts"))
                .subcommand(
                    Command::new("status")
                        .about("Inspect installed apps and active font patches")
                        .arg(target()),
                )
                .subcommand(Command::new("list").about("List built-in and custom applications")),
        )
        .subcommand(
            Command::new("config")
                .about("Manage configuration")
                .subcommand_required(true)
                .subcommand(Command::new("path").about("Print the configuration file path"))
                .subcommand(Command::new("show").about("Show effective settings as TOML"))
                .subcommand(
                    Command::new("init")
                        .about("Create annotated defaults without overwriting an existing file"),
                ),
        )
        .subcommand(
            Command::new("completion")
                .about("Generate shell completion scripts")
                .subcommand_required(true)
                .subcommand(
                    Command::new("script")
                        .about("Print a completion script for the selected shell")
                        .arg(Arg::new("shell").required(true).value_parser([
                            "bash",
                            "zsh",
                            "fish",
                            "powershell",
                        ]))
                        .arg(
                            Arg::new("no-descriptions")
                                .long("no-descriptions")
                                .action(ArgAction::SetTrue)
                                .help("Omit completion descriptions"),
                        ),
                ),
        )
        .subcommand(
            Command::new("self")
                .about("Manage this executable")
                .subcommand_required(true)
                .subcommand(
                    Command::new("upgrade")
                        .about("Install a release from an explicitly configured Rust channel"),
                ),
        );
    cli_command_kit::setup(root, mode, &help_options()).expect("fixed command tree is valid")
}
fn font_value(value: &str) -> std::result::Result<String, String> {
    if value.trim().is_empty() || value.contains('\0') {
        Err("font must be nonempty and contain no NUL".into())
    } else {
        Ok(value.into())
    }
}
