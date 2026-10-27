use better_fonts_cli::cli;
use cli_command_kit::Mode;
#[test]
fn operational_documents_cover_the_native_command_tree_and_printed_skill_location() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let folder = root.join("skills/better-fonts");
    let skill = std::fs::read_to_string(folder.join("SKILL.md")).unwrap();
    assert!(skill.contains("name: better-fonts\n"));
    assert!(cli::SKILL.contains("--skill better-fonts\n"));
    assert!(cli::SKILL.contains("/skills/better-fonts\n"));
    let mut text = skill;
    for file in [
        "commands.md",
        "configuration.md",
        "operations.md",
        "help.md",
    ] {
        text.push_str(&std::fs::read_to_string(folder.join("references").join(file)).unwrap());
    }
    let omissions =
        cli_command_kit::skill_omissions(&cli::command(Mode::Agent), &cli::help_options(), &text);
    assert!(omissions.is_empty(), "{}", omissions.join("\n"));
    let queries = std::fs::read_to_string(folder.join("references/help.md")).unwrap();
    for line in queries
        .lines()
        .filter(|line| line.starts_with("- `better-fonts "))
    {
        let command = line.trim_start_matches("- `").trim_end_matches('`');
        for mode in [Mode::Human, Mode::Agent] {
            let error = cli::command(mode)
                .try_get_matches_from(command.split_whitespace())
                .unwrap_err();
            assert!(!error.use_stderr(), "{command}: {error}");
            assert_eq!(error.to_string().starts_with("ALERT:"), mode == Mode::Agent);
        }
    }
}
