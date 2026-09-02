use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches, Parser};
use concat_with::concat_line;
use terminal_size::terminal_size;

use crate::manifest::DisplayMode;

const APP_NAME: &str = "Favicon Generator";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const AFTER_HELP: &str = "Enjoy it! https://magiclen.org";

const APP_ABOUT: &str = concat!(
    "It helps you generate favicons with different formats and sizes.\n\nEXAMPLES:\n",
    concat_line!(prefix "favicon-generator ",
        "/path/to/image /path/to/folder   # Uses /path/to/image to generate favicons into /path/to/folder",
    )
);

#[derive(Debug, Parser)]
#[command(name = APP_NAME)]
#[command(term_width = terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))]
#[command(version = CARGO_PKG_VERSION)]
#[command(author = CARGO_PKG_AUTHORS)]
#[command(after_help = AFTER_HELP)]
pub struct CLIArgs {
    #[arg(value_hint = clap::ValueHint::FilePath)]
    #[arg(help = "Assign an image for generating favicons. It should be a path of a file")]
    pub input_path: PathBuf,

    #[arg(value_hint = clap::ValueHint::DirPath)]
    #[arg(
        help = "Assign a destination of your generated files. It should be a path of a directory"
    )]
    pub output_path: PathBuf,

    #[arg(short = 'y', long)]
    #[arg(help = "Overwrite existing files without asking")]
    pub overwrite: bool,

    #[arg(long)]
    #[arg(default_value = "/", value_parser = parse_path_prefix)]
    #[arg(help = "Specify the path prefix of your favicon files")]
    pub path_prefix: String,

    #[arg(long)]
    #[arg(help = "Disable the automatic sharpening")]
    pub no_sharpen: bool,

    #[arg(long)]
    #[arg(default_value = "App")]
    #[arg(help = "Assign a name for your web app")]
    pub app_name: String,

    #[arg(long)]
    #[arg(help = "Assign a short name for your web app")]
    pub app_short_name: Option<String>,

    #[arg(long)]
    #[arg(default_value = "/")]
    #[arg(help = "Assign the URL your web app starts at")]
    pub start_url: String,

    #[arg(long)]
    #[arg(default_value = "standalone")]
    #[arg(help = "Assign how much browser UI your web app keeps")]
    pub display: DisplayMode,

    #[arg(long)]
    #[arg(default_value = "#ffffff", value_parser = parse_hex_color)]
    #[arg(help = "Assign the background color of the Apple touch icon and the maskable icon")]
    pub background_color: String,

    #[arg(long, value_parser = parse_hex_color)]
    #[arg(help = "Assign the color the browser UI is tinted with")]
    pub theme_color: Option<String>,
}

/// Make sure the prefix ends with a slash, because a file name is appended to it directly.
fn parse_path_prefix(path_prefix: &str) -> Result<String, String> {
    if path_prefix.is_empty() || path_prefix.ends_with('/') {
        return Ok(path_prefix.to_string());
    }

    Ok(format!("{path_prefix}/"))
}

/// Read a `#rgb` or a `#rrggbb` color, and normalize it into the `#rrggbb` form which both CSS and **ImageMagick** understand.
fn parse_hex_color(color: &str) -> Result<String, String> {
    let digits = match color.strip_prefix('#') {
        Some(digits) if digits.chars().all(|c| c.is_ascii_hexdigit()) => digits,
        _ => return Err(format!("{color:?} is not a color like `#ffffff`.")),
    };

    match digits.len() {
        3 => Ok(digits.chars().fold(String::from("#"), |mut color, digit| {
            let digit = digit.to_ascii_lowercase();

            color.push(digit);
            color.push(digit);

            color
        })),
        6 => Ok(format!("#{}", digits.to_ascii_lowercase())),
        _ => Err(format!("{color:?} is not a color like `#ffffff`.")),
    }
}

pub fn get_args() -> CLIArgs {
    let args = CLIArgs::command();

    let about = format!("{APP_NAME} {CARGO_PKG_VERSION}\n{CARGO_PKG_AUTHORS}\n{APP_ABOUT}");

    let args = args.about(about);

    let matches = args.get_matches();

    match CLIArgs::from_arg_matches(&matches) {
        Ok(args) => args,
        Err(err) => {
            err.exit();
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_prefix_gets_a_trailing_slash() {
        assert_eq!(Ok(String::from("/static/")), parse_path_prefix("/static"));
        assert_eq!(Ok(String::from("/static/")), parse_path_prefix("/static/"));
        assert_eq!(Ok(String::from("/")), parse_path_prefix("/"));
        assert_eq!(Ok(String::new()), parse_path_prefix(""));
    }

    #[test]
    fn hex_color_gets_normalized() {
        assert_eq!(Ok(String::from("#ffffff")), parse_hex_color("#FFF"));
        assert_eq!(Ok(String::from("#1e40af")), parse_hex_color("#1E40AF"));
    }

    #[test]
    fn a_color_which_is_not_hexadecimal_is_rejected() {
        assert!(parse_hex_color("white").is_err());
        assert!(parse_hex_color("#12345").is_err());
    }

    #[test]
    fn the_command_is_valid() {
        CLIArgs::command().debug_assert();
    }
}
