use super::{App, Driver};
use crate::{Result, config::Config, injection, invalid};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub fn registry(config: &Config, applications: &Path, home: Option<&Path>) -> Result<Vec<App>> {
    let mut apps = Vec::new();
    for (id, name, driver, preload) in [
        ("paseo", "Paseo", Driver::Electron, "dist/preload.js"),
        ("signal", "Signal", Driver::Electron, "preload.bundle.js"),
        ("slack", "Slack", Driver::Electron, "dist/preload.bundle.js"),
        ("rekordbox", "Rekordbox", Driver::NativeHook, ""),
        ("engine-dj", "Engine DJ", Driver::NativeHook, ""),
        ("telegram", "Telegram", Driver::NativeHook, ""),
        (
            "chatgpt",
            "ChatGPT",
            Driver::Electron,
            ".vite/build/preload.js",
        ),
        (
            "claude",
            "Claude",
            Driver::Electron,
            ".vite/build/mainView.js",
        ),
    ] {
        let mut path = applications.join(format!("{name}.app"));
        if id == "rekordbox" {
            path = [
                "rekordbox 7/rekordbox.app",
                "rekordbox 6/rekordbox.app",
                "rekordbox.app",
            ]
            .iter()
            .map(|v| applications.join(v))
            .find(|p| p.exists())
            .unwrap_or_else(|| applications.join("rekordbox 7/rekordbox.app"));
        }
        if ["chatgpt", "claude"].contains(&id)
            && !path.exists()
            && let Some(home) = home
        {
            let candidate = home.join("Applications").join(format!("{name}.app"));
            if candidate.exists() {
                path = candidate;
            }
        }
        let mut asar_path = PathBuf::from("app.asar");
        if id == "slack" {
            let arch = if cfg!(target_arch = "x86_64") {
                "x64"
            } else {
                "arm64"
            };
            let candidate = PathBuf::from(format!("app-{arch}.asar"));
            if path.join("Contents/Resources").join(&candidate).exists()
                || !path.join("Contents/Resources/app.asar").exists()
            {
                asar_path = candidate;
            }
        }
        apps.push(App {
            id: id.into(),
            name: name.into(),
            path,
            process_name: if id == "rekordbox" {
                "rekordbox".into()
            } else {
                name.into()
            },
            driver,
            asar_path,
            preload_path: preload.into(),
        });
    }
    for custom in &config.custom_apps {
        injection::strip("", &custom.id)?;
        if apps
            .iter()
            .any(|app| app.id.eq_ignore_ascii_case(&custom.id))
        {
            return Err(invalid(format!("duplicate application ID: {}", custom.id)));
        }
        let driver = custom.driver.parse()?;
        if custom.path.as_os_str().is_empty()
            || (driver == Driver::Electron && custom.preload_path.is_empty())
        {
            return Err(invalid(
                "custom apps require path and Electron preload_path",
            ));
        }
        apps.push(App {
            id: custom.id.clone(),
            name: custom.name.clone(),
            path: custom.path.clone(),
            process_name: custom.process_name.clone(),
            driver,
            asar_path: if custom.asar_path.as_os_str().is_empty() {
                "app.asar".into()
            } else {
                custom.asar_path.clone()
            },
            preload_path: custom.preload_path.clone(),
        });
    }
    Ok(apps)
}
pub fn select(
    registry: &[App],
    queries: &[String],
    config: &Config,
    forced: Option<Driver>,
) -> Result<Vec<App>> {
    let queries = if queries.is_empty() {
        &config.apps
    } else {
        queries
    };
    let mut selected = Vec::new();
    for query in queries {
        if ["*", "all"].contains(&query.trim().to_ascii_lowercase().as_str()) {
            selected.extend_from_slice(registry);
            continue;
        }
        let matches: Vec<_> = registry
            .iter()
            .filter(|a| {
                a.id.eq_ignore_ascii_case(query.trim())
                    || a.name.eq_ignore_ascii_case(query.trim())
                    || a.path
                        .to_str()
                        .is_some_and(|path| path.eq_ignore_ascii_case(query.trim()))
            })
            .collect();
        if matches.len() > 1 {
            return Err(invalid(format!("ambiguous application: {query}")));
        }
        if let Some(app) = matches.first() {
            selected.push((*app).clone());
            continue;
        }
        let path = if query.ends_with(".app") {
            std::path::absolute(query)?
        } else {
            Path::new("/Applications").join(format!("{query}.app"))
        };
        if !path.is_dir() {
            return Err(invalid(format!("unknown or missing application: {query}")));
        }
        let mut archives = Vec::new();
        if let Ok(entries) = fs::read_dir(path.join("Contents/Resources")) {
            for entry in entries {
                let entry = entry?;
                if entry.path().extension().is_some_and(|v| v == "asar") {
                    archives.push(entry.file_name());
                }
            }
        }
        let driver = forced.unwrap_or(if archives.is_empty() {
            Driver::NativeHook
        } else {
            Driver::Electron
        });
        if driver == Driver::Electron && archives.len() != 1 {
            return Err(invalid(
                "arbitrary Electron app requires exactly one ASAR; configure custom_apps for other layouts",
            ));
        }
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let id = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        selected.push(App {
            id,
            name: name.clone(),
            path,
            process_name: name,
            driver,
            asar_path: archives
                .first()
                .map(PathBuf::from)
                .unwrap_or_else(|| "app.asar".into()),
            preload_path: "preload.js".into(),
        });
    }
    let mut unique = Vec::new();
    for mut app in selected {
        if let Some(driver) = forced {
            app.driver = driver;
        }
        if !unique.iter().any(|a: &App| a.path == app.path) {
            unique.push(app);
        }
    }
    Ok(unique)
}
