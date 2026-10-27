use super::{Environment, record};
use crate::{
    Error, Result,
    app::{self, Apply, Driver},
    config::Config,
    invalid,
    process::Runner,
};
use std::{io::Write, path::Path};
use tokio_util::sync::CancellationToken;
pub(super) fn execute(
    subject: &clap::ArgMatches,
    path: &Path,
    environment: &Environment,
    runner: &dyn Runner,
    output: &mut impl Write,
    token: &CancellationToken,
    verbose: bool,
) -> Result<()> {
    let config = Config::ensure(path)?;
    let apps = app::registry(
        &config,
        std::path::Path::new("/Applications"),
        environment.home.as_deref(),
    )?;
    let (operation, leaf) = subject.subcommand().unwrap();
    let queries = leaf
        .try_get_many::<String>("apps")
        .ok()
        .flatten()
        .map(|v| v.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    if operation == "list" {
        for app in apps {
            record(
                output,
                environment,
                &serde_json::json!({"app":app.name,"id":app.id,"driver":app.driver,"path":app.path}),
                &format!("{} ({:?}) {}", app.name, app.driver, app.path.display()),
            )?;
        }
        return Ok(());
    }
    let all = if operation == "status" && queries.is_empty() {
        apps.clone()
    } else {
        app::select(
            &apps,
            &queries,
            &config,
            leaf.try_get_one::<Driver>("driver").ok().flatten().copied(),
        )?
    };
    let mut failures = Vec::new();
    let mut committed = false;
    for app in all {
        if operation == "status" {
            match app.status() {
                Ok(status) => record(
                    output,
                    environment,
                    &serde_json::json!({"app":app.name,"id":app.id,"driver":app.driver,"path":app.path,"installed":status.installed,"patched":status.patched,"font":status.font}),
                    &format!(
                        "{}: installed={} patched={} font={} {}",
                        app.name,
                        status.installed,
                        status.patched,
                        status.font.as_deref().unwrap_or("-"),
                        app.path.display()
                    ),
                )?,
                Err(e) => failures.push(format!("{}: {e}", app.name)),
            }
            continue;
        }
        let font = leaf
            .try_get_one::<String>("font")
            .ok()
            .flatten()
            .map(String::as_str);
        let restart = if leaf.get_flag("no-restart") {
            Some(false)
        } else {
            leaf.get_one::<bool>("restart").copied()
        };
        if !app.path.exists() {
            record(
                output,
                environment,
                &serde_json::json!({"app":app.name,"state":"not-installed","path":app.path}),
                &format!("{}: not installed ({})", app.name, app.path.display()),
            )?;
            continue;
        }
        let options = Apply {
            font: config.font_for(&app.id, font),
            patch: operation == "patch",
            restart: config.restart_for(&app.id, restart),
            dry_run: leaf.get_flag("dry-run"),
            cache: &environment.cache,
        };
        match app.apply(&options, runner, token) {
            Ok(()) => {
                let state = if options.dry_run {
                    "dry-run"
                } else if options.patch {
                    "patched"
                } else {
                    "unpatched"
                };
                record(
                    output,
                    environment,
                    &serde_json::json!({"app":app.name,"state":state,"path":app.path,"font":options.font,"restart":options.restart}),
                    &format!(
                        "{}: {state}{}",
                        app.name,
                        if verbose {
                            format!(
                                " font={} restart={} {}",
                                options.font,
                                options.restart,
                                app.path.display()
                            )
                        } else {
                            String::new()
                        }
                    ),
                )?;
            }
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(e) => {
                committed |= matches!(e, Error::Committed(_));
                failures.push(format!("{}: {e}", app.name));
            }
        }
    }
    if !failures.is_empty() {
        return Err(if committed {
            Error::Committed(failures.join("; "))
        } else {
            invalid(failures.join("; "))
        });
    }
    Ok(())
}
