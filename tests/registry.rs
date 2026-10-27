use better_fonts_cli::{
    app::{self, Driver},
    config::{Config, CustomApp},
};
use std::fs;
fn temp() -> tempfile::TempDir {
    tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap()
}
#[test]
fn builtins_use_actual_installations_and_architecture_archive_then_standard_fallback() {
    let dir = temp();
    let apps = dir.path().join("Applications");
    let home = dir.path().join("home");
    fs::create_dir_all(apps.join("rekordbox 6/rekordbox.app")).unwrap();
    fs::create_dir_all(home.join("Applications/ChatGPT.app")).unwrap();
    fs::create_dir_all(apps.join("Slack.app/Contents/Resources")).unwrap();
    fs::write(
        apps.join("Slack.app/Contents/Resources/app.asar"),
        b"fixture",
    )
    .unwrap();
    let cfg = Config::default();
    let registry = app::registry(&cfg, &apps, Some(&home)).unwrap();
    assert_eq!(registry.len(), 8);
    assert!(
        registry
            .iter()
            .find(|a| a.id == "rekordbox")
            .unwrap()
            .path
            .ends_with("rekordbox 6/rekordbox.app")
    );
    assert!(
        registry
            .iter()
            .find(|a| a.id == "chatgpt")
            .unwrap()
            .path
            .starts_with(&home)
    );
    assert_eq!(
        registry
            .iter()
            .find(|a| a.id == "slack")
            .unwrap()
            .asar_path
            .to_str(),
        Some("app.asar")
    );
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        "arm64"
    };
    fs::write(
        apps.join(format!("Slack.app/Contents/Resources/app-{arch}.asar")),
        b"fixture",
    )
    .unwrap();
    assert_eq!(
        app::registry(&cfg, &apps, None)
            .unwrap()
            .iter()
            .find(|a| a.id == "slack")
            .unwrap()
            .asar_path
            .to_str(),
        Some(format!("app-{arch}.asar").as_str())
    );
    let selected = app::select(&registry, &[" SLACK ".into(), "Slack".into()], &cfg, None).unwrap();
    assert_eq!(selected.len(), 1);
    let slack = registry.iter().find(|app| app.id == "slack").unwrap();
    assert_eq!(
        app::select(
            &registry,
            &[slack.path.to_str().unwrap().to_uppercase()],
            &cfg,
            None
        )
        .unwrap()[0]
            .id,
        "slack"
    );
    assert!(
        app::select(&registry, &["all".into()], &cfg, Some(Driver::NativeHook))
            .unwrap()
            .iter()
            .all(|a| a.driver == Driver::NativeHook)
    );
    assert_eq!(
        app::select(
            &registry,
            &[],
            &Config {
                apps: vec!["signal".into()],
                ..cfg.clone()
            },
            None
        )
        .unwrap()[0]
            .id,
        "signal"
    );
    assert!(app::select(&registry, &["missing-not-real".into()], &cfg, None).is_err());
}
#[test]
fn custom_apps_validate_identity_driver_and_required_layout_and_arbitrary_paths_are_discovered() {
    let dir = temp();
    let applications = dir.path().join("Applications");
    let path = dir.path().join("Custom.app");
    fs::create_dir_all(path.join("Contents/Resources")).unwrap();
    fs::write(path.join("Contents/Resources/other.asar"), b"data").unwrap();
    let mut cfg = Config {
        custom_apps: vec![CustomApp {
            id: "custom".into(),
            name: "Custom".into(),
            path: path.clone(),
            process_name: "Custom".into(),
            preload_path: "preload.js".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let registry = app::registry(&cfg, &applications, None).unwrap();
    assert_eq!(
        registry.last().unwrap().asar_path.to_str(),
        Some("app.asar")
    );
    let selected = app::select(&[], &[path.display().to_string()], &cfg, None).unwrap();
    assert_eq!(selected[0].asar_path.to_str(), Some("other.asar"));
    assert_eq!(selected[0].driver, Driver::Electron);
    fs::write(path.join("Contents/Resources/second.asar"), b"data").unwrap();
    assert!(app::select(&[], &[path.display().to_string()], &cfg, None).is_err());
    cfg.custom_apps[0].id = "slack".into();
    assert!(app::registry(&cfg, &applications, None).is_err());
    cfg.custom_apps[0].id = "bad/id".into();
    assert!(app::registry(&cfg, &applications, None).is_err());
    cfg.custom_apps[0].id = "custom".into();
    cfg.custom_apps[0].driver = "wrong".into();
    assert!(app::registry(&cfg, &applications, None).is_err());
    cfg.custom_apps[0].driver = "electron".into();
    cfg.custom_apps[0].preload_path.clear();
    assert!(app::registry(&cfg, &applications, None).is_err());
    fs::remove_dir_all(path.join("Contents/Resources")).unwrap();
    assert_eq!(
        app::select(&[], &[path.display().to_string()], &Config::default(), None).unwrap()[0]
            .driver,
        Driver::NativeHook
    );
}
