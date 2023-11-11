use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches, Parser};
use concat_with::concat_line;
use terminal_size::terminal_size;

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
    #[arg(help = "Overwrite exiting files without asking")]
    pub overwrite: bool,

    #[arg(long)]
    #[arg(default_value = "/")]
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
