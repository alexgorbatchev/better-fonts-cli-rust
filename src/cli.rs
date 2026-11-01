use crate::{
    Error, Result,
    config::{self, Config},
    invalid,
    process::{Runner, SystemRunner},
};
use clap::Command;
use cli_command_kit::Mode;
use std::{collections::BTreeMap, ffi::OsString, io::Write, path::PathBuf};
use tokio_util::sync::CancellationToken;
mod apps;
mod command;
pub use command::{command, help_options};
pub struct Environment {
    pub mode: Mode,
    pub home: Option<PathBuf>,
    pub config: PathBuf,
    pub cache: PathBuf,
    pub release: BTreeMap<String, String>,
}
impl Environment {
    pub fn read() -> Result<Self> {
        Ok(Self {
            mode: Mode::from_env(),
            home: dirs::home_dir(),
            config: config::config_path()?,
            cache: config::cache_dir()?,
            release: std::env::vars()
                .filter(|(k, _)| k.starts_with("BETTER_FONTS_RUST_RELEASE_"))
                .collect(),
        })
    }
    fn display(&self, value: &str) -> String {
        self.home.as_deref().map_or_else(
            || value.to_owned(),
            |home| cli_command_kit::fold_message(value, home),
        )
    }
}
pub async fn execute<I, T>(
    args: I,
    environment: &Environment,
    output: &mut impl Write,
) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let token = CancellationToken::new();
    let cancel = token.clone();
    let signal = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            cancel.cancel();
        }
    });
    let result = execute_with(
        args,
        environment,
        &SystemRunner {
            scratch: environment.cache.join("process"),
        },
        output,
        &token,
    )
    .await;
    signal.abort();
    result
}
pub async fn execute_with<I, T>(
    args: I,
    environment: &Environment,
    runner: &dyn Runner,
    output: &mut impl Write,
    token: &CancellationToken,
) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut command = command(environment.mode);
    let matches = match command.try_get_matches_from_mut(args) {
        Ok(matches) => matches,
        Err(error) if !error.use_stderr() => {
            write!(output, "{}", environment.display(&error.to_string()))?;
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };
    if matches.get_flag("version") {
        writeln!(output, "{}", env!("CARGO_PKG_VERSION"))?;
        return Ok(());
    }
    if cli_command_kit::write_skill(&command, &matches, &help_options(), output)? {
        return Ok(());
    }
    let path = matches
        .get_one::<PathBuf>("config")
        .unwrap_or(&environment.config);
    match matches.subcommand() {
        Some(("completion", subject)) => {
            use clap_complete::Generator;
            let leaf = subject.subcommand().unwrap().1;
            let shell: clap_complete::Shell = leaf
                .get_one::<String>("shell")
                .unwrap()
                .parse()
                .map_err(invalid)?;
            let mut tree = if leaf.get_flag("no-descriptions") {
                without_descriptions(command)
            } else {
                command
            };
            tree.set_bin_name("better-fonts");
            tree.build();
            shell.try_generate(&tree, output)?;
        }
        Some(("config", subject)) => match subject.subcommand_name() {
            Some("path") => writeln!(
                output,
                "{}",
                environment.display(&path.display().to_string())
            )?,
            Some("init") => {
                Config::ensure(path)?;
                writeln!(
                    output,
                    "{}",
                    environment.display(&path.display().to_string())
                )?;
            }
            Some("show") => {
                let config = Config::ensure(path)?;
                write!(
                    output,
                    "{}",
                    environment.display(
                        &toml::to_string_pretty(&config).map_err(|e| invalid(e.to_string()))?
                    )
                )?;
            }
            _ => unreachable!(),
        },
        Some(("self", _)) => {
            let update = updater(environment)?
                .update_running(env!("CARGO_PKG_VERSION"), token)
                .await?;
            writeln!(output, "{update:?}")?;
        }
        Some(("app", subject)) => apps::execute(
            subject,
            path,
            environment,
            runner,
            output,
            token,
            matches.get_flag("verbose"),
        )?,

        _ => unreachable!(),
    }
    Ok(())
}
fn without_descriptions(command: Command) -> Command {
    command
        .about(None::<&'static str>)
        .long_about(None::<&'static str>)
        .mut_args(|argument| {
            argument
                .help(None::<&'static str>)
                .long_help(None::<&'static str>)
        })
        .mut_subcommands(without_descriptions)
}
fn record(
    output: &mut impl Write,
    environment: &Environment,
    value: &serde_json::Value,
    human: &str,
) -> Result<()> {
    let text = if environment.mode == Mode::Agent {
        serde_json::to_string(value)?
    } else {
        human.to_owned()
    };
    writeln!(output, "{}", environment.display(&text))?;
    Ok(())
}
pub fn updater(environment: &Environment) -> Result<cli_deps::Updater> {
    let keys = [
        "BETTER_FONTS_RUST_RELEASE_BASE_URL",
        "BETTER_FONTS_RUST_RELEASE_OWNER",
        "BETTER_FONTS_RUST_RELEASE_REPO",
    ];
    let values: Vec<_> = keys
        .iter()
        .map(|key| environment.release.get(*key).filter(|v| !v.is_empty()))
        .collect();
    if values.iter().all(|v| v.is_none()) {
        return Ok(cli_deps::Updater::default());
    }
    if values.iter().any(|v| v.is_none()) {
        return Err(invalid(
            "configure all three BETTER_FONTS_RUST_RELEASE_BASE_URL, BETTER_FONTS_RUST_RELEASE_OWNER and BETTER_FONTS_RUST_RELEASE_REPO",
        ));
    }
    Ok(cli_deps::Updater {
        channel: Some(cli_deps::ReleaseChannel::new(
            values[0].unwrap(),
            values[1].unwrap().as_str(),
            values[2].unwrap().as_str(),
        )?),
    })
}
pub fn exit_status(error: &Error) -> i32 {
    match error {
        Error::Parse(_) => 2,
        Error::Cancelled => 130,
        Error::Committed(_) => 6,
        _ => 1,
    }
}
