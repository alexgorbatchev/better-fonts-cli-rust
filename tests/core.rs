use better_fonts_cli::{archive::Archive, config::Config, injection};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path};

fn fixture(path: &Path, header: Value, payload: &[u8]) {
    let json = serde_json::to_vec(&header).unwrap();
    let padded = (json.len() + 3) & !3;
    let mut f = fs::File::create(path).unwrap();
    for n in [4, padded as u32 + 8, padded as u32 + 4, json.len() as u32] {
        f.write_all(&n.to_le_bytes()).unwrap();
    }
    f.write_all(&json).unwrap();
    f.write_all(&vec![0; padded - json.len()]).unwrap();
    f.write_all(payload).unwrap();
}

#[test]
fn archive_replace_preserves_metadata_unpacked_links_empty_directories_and_integrity() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let path = dir.path().join("app.asar");
    fixture(
        &path,
        json!({"custom":{"value":3},"files":{
            "preload.js":{"size":3,"offset":"0","executable":true,"vendor":9,"integrity":{"algorithm":"SHA256","blockSize":2,"hash":"old","blocks":[],"vendor":4}},
            "data":{"size":4,"offset":"3","integrity":{"hash":"untouched"}},
            "empty":{"files":{}},"link":{"link":"data"},"external":{"unpacked":true,"size":99}
        }}),
        b"oldDATA",
    );
    let mut archive = Archive::open(&path).unwrap();
    assert_eq!(archive.extract("data").unwrap(), b"DATA");
    archive.replace("preload.js", b"changed").unwrap();
    let mut result = Archive::open(&path).unwrap();
    assert_eq!(result.extract("preload.js").unwrap(), b"changed");
    assert_eq!(result.extract("data").unwrap(), b"DATA");
    assert_eq!(result.header()["custom"]["value"], 3);
    assert_eq!(result.header()["files"]["preload.js"]["vendor"], 9);
    assert_eq!(
        result.header()["files"]["preload.js"]["integrity"]["vendor"],
        4
    );
    assert_eq!(
        result.header()["files"]["preload.js"]["integrity"]["blocks"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        result.header()["files"]["data"]["integrity"]["hash"],
        "untouched"
    );
    assert_eq!(result.header()["files"]["empty"], json!({"files":{}}));
    assert!(result.extract("external").is_err());
    assert!(result.extract("link").is_err());
    assert!(result.replace("missing", b"bad").is_err());
}

#[test]
fn malformed_archive_is_rejected_without_modifying_bytes() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let path = dir.path().join("app.asar");
    for header in [
        json!({"files":{"a":{"size":-1,"offset":"0"}}}),
        json!({"files":{"a":{"size":5,"offset":"9999999999999999999999999"}}}),
        json!({"files":{"../a":{"size":0,"offset":"0"}}}),
        json!({"files":{"a":{"size":4,"offset":"0"}}}),
    ] {
        fixture(&path, header, b"x");
        let before = fs::read(&path).unwrap();
        assert!(Archive::open(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    fs::write(&path, b"bad").unwrap();
    assert!(Archive::open(&path).is_err());
}

#[test]
fn injection_roundtrips_exact_preload_and_escapes_font_without_deleting_other_code() {
    let original = "const unrelated='font-family: wrong';\n\n";
    let font = "Quote\"\\\n');throw new Error('bad');/*";
    let patched = injection::inject(original, "demo", font).unwrap();
    assert_eq!(
        injection::detect(&patched, "demo").unwrap(),
        Some(font.to_owned())
    );
    assert_eq!(injection::inject(&patched, "demo", font).unwrap(), patched);
    assert_eq!(injection::strip(&patched, "demo").unwrap(), original);
    assert_eq!(
        injection::strip("/*fonted-demo-patch*/unrelated();", "demo").unwrap(),
        "/*fonted-demo-patch*/unrelated();"
    );
    assert!(injection::inject(original, "../bad", font).is_err());
    assert!(injection::inject(original, "demo", "bad\0font").is_err());
}

#[test]
fn existing_electron_patch_is_detected_replaced_and_removed_without_neighboring_code_loss() {
    let original = "const earlier='font-family: wrong';";
    let css = "*:not([data-cds*=\"Icon\"]):not([data-cds*=\"Icon\"] *):not([class*=\"icon\" i]):not([class*=\"Icon\"]):not([data-icon]):not(i):not(svg):not(svg *) { font-family: \"Old Font\", monospace !important; } [data-cds*=\"Icon\"], [data-cds*=\"Icon\"] * { font-family: var(--font-anthropicons, Anthropicons-Variable), sans-serif !important; }";
    let patched = format!(
        "{original}\n;/*fonted-demo-patch*/(function(){{document.addEventListener(\"DOMContentLoaded\",function(){{var s=document.createElement(\"style\");s.textContent='{css}{}",
        "';document.head.appendChild(s)})})();\n"
    );
    assert_eq!(
        injection::detect(&patched, "demo").unwrap().as_deref(),
        Some("Old Font")
    );
    assert_eq!(injection::strip(&patched, "demo").unwrap(), original);
    let replaced = injection::inject(&patched, "demo", "New Font").unwrap();
    assert_eq!(
        injection::detect(&replaced, "demo").unwrap().as_deref(),
        Some("New Font")
    );
    assert_eq!(replaced.matches("fonted-demo-patch*/").count(), 1);
}

#[test]
fn config_defaults_overrides_and_creation_preserve_existing_configuration() {
    let dir = tempfile::tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/.tmp")).unwrap();
    let path = dir.path().join("nested/config.toml");
    let default = Config::ensure(&path).unwrap();
    assert!(default.matches("slack"));
    fs::write(&path,"font='Global'\napps=[' Slack ']\nrestart=false\n[apps_config.slack]\nfont='Per App'\nrestart=true\n").unwrap();
    let cfg = Config::ensure(&path).unwrap();
    assert!(cfg.matches("SLACK"));
    assert!(!cfg.matches("signal"));
    assert_eq!(cfg.font_for("slack", None), "Per App");
    assert_eq!(cfg.font_for("slack", Some("Flag")), "Flag");
    assert!(cfg.restart_for("slack", None));
    assert!(!cfg.restart_for("signal", None));
    assert!(!cfg.restart_for("slack", Some(false)));
    cfg.save(&path).unwrap();
    assert_eq!(Config::load(&path).unwrap().font, "Global");
    fs::write(&path, "[not valid").unwrap();
    assert!(Config::ensure(&path).is_err());
    fs::write(
        &path,
        "font=''\napps=[]\nrestart=false\n[apps_config.demo]\nfont='  '\n",
    )
    .unwrap();
    let config = Config::load(&path).unwrap();
    assert_eq!(config.font_for("demo", None), "Maple Mono Normal NF");
    assert!(config.matches("anything"));
    assert!(!config.restart_for("demo", None));
    let blank = Config {
        font: "   ".into(),
        ..Config::default()
    };
    assert_eq!(blank.font_for("demo", None), "Maple Mono Normal NF");
    let obstructed = dir.path().join("not-a-directory");
    fs::write(&obstructed, b"file").unwrap();
    assert!(Config::ensure(&obstructed.join("config")).is_err());
}

#[test]
fn malformed_injections_fail_closed_and_existing_js_escapes_are_parsed() {
    for text in [
        "\n;/*fonted-demo-patch*/(function(){var __better_fonts_font=\"Font\";",
        "\n;/*fonted-demo-patch*/(function(){document.addEventListener(\"DOMContentLoaded\",function(){var s=document.createElement(\"style\");s.textContent='broken\\x';document.head.appendChild(s)})})();\n",
        "\n;/*fonted-demo-patch*/(function(){document.addEventListener(\"DOMContentLoaded\",function(){var s=document.createElement(\"style\");s.textContent='unterminated",
    ] {
        assert!(injection::strip(text, "demo").is_err());
    }
    let prefix = "\n;/*fonted-demo-patch*/(function(){document.addEventListener(\"DOMContentLoaded\",function(){var s=document.createElement(\"style\");s.textContent='";
    let snippet = format!(
        "{prefix}* {{ font-family: \"O\\'Brien\\\\Mono\\n\", monospace !important; }}{}",
        "';document.head.appendChild(s)})})();\n"
    );
    assert_eq!(
        injection::detect(&snippet, "demo").unwrap().as_deref(),
        Some("O'Brien\\Mono\n")
    );
    assert_eq!(injection::strip(&snippet, "demo").unwrap(), "");
}
