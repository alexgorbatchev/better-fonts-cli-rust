use better_fonts_cli::{Error, cli};
use std::io::Write;
#[tokio::main]
async fn main() {
    let result = match cli::Environment::read() {
        Ok(environment) => {
            cli::execute(std::env::args_os(), &environment, &mut std::io::stdout()).await
        }
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        let status = cli::exit_status(&error);
        let text = match &error {
            Error::Parse(error) => error.to_string(),
            _ => format!("ERR: {error}\n"),
        };
        let text = cli_command_kit::display_message(&text);
        let _ = std::io::stderr().write_all(text.as_bytes());
        std::process::exit(status);
    }
}
