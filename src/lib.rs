//! # Favicon Generator
//! It helps you generate favicons with different formats and sizes.

extern crate clap;
extern crate terminal_size;
extern crate image_convert;
extern crate scanner_rust;
#[macro_use]
extern crate validators;
extern crate subprocess;
#[macro_use]
extern crate serde_json;
extern crate tera;
#[macro_use]
extern crate lazy_static;

use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, ErrorKind, Write};

use terminal_size::{Width, terminal_size};
use clap::{App, Arg};

use image_convert::{ColorName, ImageResource, PGMConfig, PNGConfig, ICOConfig, identify, to_pgm, to_ico, to_png, magick_rust::{bindings, PixelWand}};

use scanner_rust::{Scanner, ScannerError};

use validators::boolean::Boolean;
use validators::regex::Regex;

use subprocess::{Exec, ExitStatus, PopenError, NullFile};

use tera::{Tera, Context};


// TODO -----Config START-----

const APP_NAME: &str = "Favicon Generator";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const DEFAULT_POTRACE_PATH: &str = "potrace";
const DEFAULT_PATH_PREFIX: &str = "/";
const DEFAULT_THRESHOLD: &str = "0.5";
const DEFAULT_BACKGROUND_COLOR: &str = "#FFFFFF";

const FILE_WEB_APP_MANIFEST: &str = "web-app.manifest";
const FILE_BROWSER_CONFIG: &str = "browser-config.xml";
const FILE_SVG_MONOCHROME: &str = "favicon-monochrome.svg";
const FILE_PNG_IOS_BACKGROUND: &str = "favicon-180-i.png";

const ICO_SIZE: [u16; 3] = [48, 32, 16];
const PNG_SIZE: [u16; 7] = [512, 310, 192, 150, 70, 32, 16];

lazy_static! {
    static ref RE_HEX_COLOR: Regex = {
        Regex::new("^#[0-f0-F]{6}$").unwrap()
    };
}

validated_customized_ranged_number!(pub Threshold, f64, 0f64, 1.0f64);
validated_customized_regex_string!(pub HexColor, ref RE_HEX_COLOR);

#[derive(Debug)]
pub struct ExePaths {
    pub potrace: PathBuf,
}

impl Default for ExePaths {
    #[inline]
    fn default() -> Self {
        ExePaths {
            potrace: DEFAULT_POTRACE_PATH.into()
        }
    }
}

impl ExePaths {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug)]
pub struct Config {
    pub paths: ExePaths,
    pub input: PathBuf,
    pub output: PathBuf,
    pub path_prefix: PathBuf,
    pub overwrite: bool,
    pub threshold: Threshold,
    pub sharpen: bool,
    pub app_name: String,
    pub app_short_name: String,
    pub android_background_color: HexColor,
    pub ios_background_color: HexColor,
    pub windows_background_color: HexColor,
}

impl Config {
    pub fn from_cli() -> Result<Config, String> {
        let arg0 = env::args().next().unwrap();
        let arg0 = Path::new(&arg0).file_stem().unwrap().to_str().unwrap();

        let examples = vec![
            "/path/to/image /path/to/folder     # Use /path/to/image to generate favicons into /path/to/folder",
        ];

        let terminal_width = if let Some((Width(width), _)) = terminal_size() {
            width as usize
        } else {
            0
        };

        let matches = App::new(APP_NAME)
            .set_term_width(terminal_width)
            .version(CARGO_PKG_VERSION)
            .author(CARGO_PKG_AUTHORS)
            .about(format!("\n\nEXAMPLES:\n{}", examples.iter()
                .map(|e| format!("  {} {}\n", arg0, e))
                .collect::<Vec<String>>()
                .concat()
            ).as_str()
            )
            .arg(Arg::with_name("INPUT_PATH")
                .required(true)
                .help("Assigns an image for generating favicons. It should be a path of a file")
                .takes_value(true)
                .display_order(0)
            )
            .arg(Arg::with_name("OUTPUT_PATH")
                .required(true)
                .help("Assigns a destination of your generated files. It should be a path of a directory")
                .takes_value(true)
                .display_order(1)
            )
            .arg(Arg::with_name("OVERWRITE")
                .long("overwrite")
                .short("y")
                .help("Overwrites exiting files without asking")
                .display_order(0)
            )
            .arg(Arg::with_name("POTRACE_PATH")
                .long("potrace-path")
                .help("Specifies the path of your potrace executable binary file")
                .takes_value(true)
                .default_value(DEFAULT_POTRACE_PATH)
                .display_order(1)
            )
            .arg(Arg::with_name("PATH_PREFIX")
                .long("path-prefix")
                .help("Specifies the path prefix of your favicon files")
                .takes_value(true)
                .default_value(DEFAULT_PATH_PREFIX)
                .display_order(2)
            )
            .arg(Arg::with_name("THRESHOLD").value_name("FLOATING_VALUE")
                .long("threshold")
                .short("t")
                .help("The black/white cutoff in input file")
                .takes_value(true)
                .default_value(DEFAULT_THRESHOLD)
                .display_order(3)
            )
            .arg(Arg::with_name("NO_SHARPEN")
                .long("no-sharpen")
                .help("Disables the automatic sharpening")
                .display_order(4)
            )
            .arg(Arg::with_name("APP_NAME").value_name("NAME")
                .long("app-name")
                .help("Assigns a name for your web app")
                .takes_value(true)
                .display_order(10)
            )
            .arg(Arg::with_name("APP_SHORT_NAME").value_name("NAME")
                .long("app-short-name")
                .help("Assigns a short name for your web app")
                .takes_value(true)
                .display_order(11)
            )
            .arg(Arg::with_name("BACKGROUND_COLOR").value_name("HEX_COLOR")
                .long("background-color").visible_alias("background")
                .help("Forces to assign a background color for all devices")
                .takes_value(true)
                .display_order(12)
            )
            .arg(Arg::with_name("ANDROID_BACKGROUND_COLOR").value_name("HEX_COLOR")
                .long("android-background-color").visible_alias("android-background")
                .help("Assigns a background color for Android devices")
                .takes_value(true)
                .default_value(DEFAULT_BACKGROUND_COLOR)
                .display_order(13)
            )
            .arg(Arg::with_name("IOS_BACKGROUND_COLOR").value_name("HEX_COLOR")
                .long("ios-background-color").visible_alias("ios-background")
                .help("Assigns a background color for iOS devices")
                .takes_value(true)
                .default_value(DEFAULT_BACKGROUND_COLOR)
                .display_order(14)
            )
            .arg(Arg::with_name("WINDOWS_BACKGROUND_COLOR").value_name("HEX_COLOR")
                .long("windows-background-color").visible_alias("windows-background")
                .help("Assigns a background color for Windows devices")
                .takes_value(true)
                .default_value(DEFAULT_BACKGROUND_COLOR)
                .display_order(15)
            )
            .after_help("Enjoy it! https://magiclen.org")
            .get_matches();

        let (android_background_color, ios_background_color, windows_background_color) = match matches.value_of("BACKGROUND_COLOR") {
            Some(background_color) => {
                let background_color = HexColor::from_str(background_color).map_err(|err| err.to_string())?;

                (background_color.clone(), background_color.clone(), background_color)
            }
            None => {
                (
                    HexColor::from_str(matches.value_of("ANDROID_BACKGROUND_COLOR").unwrap()).map_err(|err| err.to_string())?,
                    HexColor::from_str(matches.value_of("IOS_BACKGROUND_COLOR").unwrap()).map_err(|err| err.to_string())?,
                    HexColor::from_str(matches.value_of("WINDOWS_BACKGROUND_COLOR").unwrap()).map_err(|err| err.to_string())?,
                )
            }
        };

        Ok(Config {
            paths: ExePaths {
                potrace: matches.value_of("POTRACE_PATH").unwrap().into()
            },
            input: matches.value_of("INPUT_PATH").unwrap().into(),
            output: matches.value_of("OUTPUT_PATH").unwrap().into(),
            path_prefix: matches.value_of("PATH_PREFIX").unwrap().into(),
            overwrite: matches.is_present("OVERWRITE"),
            threshold: Threshold::from_str(matches.value_of("THRESHOLD").unwrap()).map_err(|err| err.to_string())?,
            sharpen: !matches.is_present("NO_SHARPEN"),
            app_name: matches.value_of("APP_NAME").map(|s| s.into()).unwrap_or(String::new()),
            app_short_name: matches.value_of("APP_SHORT_NAME").map(|s| s.into()).unwrap_or(String::new()),
            android_background_color,
            ios_background_color,
            windows_background_color,
        })
    }
}

// TODO -----Config END-----

// TODO -----Process START-----

fn check_executable(cmd: &[&str]) -> Result<(), ()> {
    let process = Exec::cmd(cmd[0]).args(&cmd[1..]).stdout(NullFile {}).stderr(NullFile {});

    match execute_join(process) {
        Ok(es) => {
            if es == 0 {
                Ok(())
            } else {
                Err(())
            }
        }
        Err(_) => Err(())
    }
}

fn execute_one_stdin(cmd: &[&str], cwd: &str, input: Vec<u8>) -> Result<i32, String> {
    if let Err(error) = fs::create_dir_all(cwd) {
        return Err(error.to_string());
    }

    let process = Exec::cmd(cmd[0]).cwd(cwd).args(&cmd[1..]).stdin(input);

    match execute_capture(process) {
        Ok(es) => Ok(es),
        Err(error) => Err(error.to_string())
    }
}

fn execute_capture(process: Exec) -> Result<i32, PopenError> {
    match process.capture() {
        Ok(capture) => {
            let err_message = capture.stderr_str();
            if !err_message.is_empty() {
                eprintln!("{}", err_message);
            }
            match capture.exit_status {
                ExitStatus::Exited(c) => Ok(c as i32),
                ExitStatus::Signaled(c) => Ok(c as i32),
                ExitStatus::Other(c) => Ok(c),
                _ => Ok(-1),
            }
        }
        Err(error) => {
            Err(error)
        }
    }
}

fn execute_join(process: Exec) -> Result<i32, PopenError> {
    match process.join() {
        Ok(es) => {
            match es {
                ExitStatus::Exited(c) => Ok(c as i32),
                ExitStatus::Signaled(c) => Ok(c as i32),
                ExitStatus::Other(c) => Ok(c),
                _ => Ok(-1),
            }
        }
        Err(error) => {
            Err(error)
        }
    }
}

// TODO -----Process END-----

pub fn run(config: Config) -> Result<i32, String> {
    let potrace = config.paths.potrace.to_str().ok_or(format!("`{}` is not a correct UTF-8 string.", config.paths.potrace.to_string_lossy()))?;

    if let Err(_) = check_executable(&vec![potrace, "-v"]) {
        return Err(format!("Cannot execute `{}`.", potrace));
    }

    let input = config.input.canonicalize().map_err(|err| err.to_string())?;

    if !input.is_file() {
        return Err(format!("`{}` is not a file.", input.to_string_lossy()));
    }

    let input_str = input.to_str().ok_or(format!("`{}` is not a correct UTF-8 string.", input.to_string_lossy()))?;

    let output_str = config.output.to_str().ok_or(format!("`{}` is not a correct UTF-8 string.", config.output.to_string_lossy()))?;

    let path_prefix = config.path_prefix.to_str().ok_or(format!("`{}` is not a correct UTF-8 string.", config.path_prefix.to_string_lossy()))?;

    let web_app_manifest = config.output.join(FILE_WEB_APP_MANIFEST);
    let browser_config = config.output.join(FILE_BROWSER_CONFIG);
    let svg_monochrome = config.output.join(FILE_SVG_MONOCHROME);
    let png_ios_background = config.output.join(FILE_PNG_IOS_BACKGROUND);
    let ico = config.output.join("favicon.ico");
    let png_vec = {
        let mut v = Vec::with_capacity(PNG_SIZE.len());

        for &size in PNG_SIZE.iter() {
            v.push(config.output.join(format!("favicon-{}.png", size)));
        }

        v
    };

    if config.output.exists() {
        let need_overwrite = {
            let mut path_vec = Vec::with_capacity(5 + PNG_SIZE.len());

            path_vec.extend_from_slice(&[&web_app_manifest, &browser_config, &svg_monochrome, &png_ios_background, &ico]);

            for png in png_vec.iter() {
                path_vec.push(png);
            }

            let mut need_overwrite = false;

            for path in path_vec {
                let metadata = path.metadata();

                match metadata {
                    Ok(metadata) => {
                        if metadata.is_file() {
                            need_overwrite = true;
                        } else {
                            return Err(format!("`{}` is not a file.", path.to_string_lossy()));
                        }
                    }
                    Err(ref err) if err.kind() == ErrorKind::NotFound => {
                        // do nothing
                    }
                    Err(err) => {
                        return Err(err.to_string());
                    }
                }
            }

            need_overwrite
        };

        if need_overwrite && !config.overwrite {
            let mut sc = Scanner::new(io::stdin());

            loop {
                print!("Overwrite files? [Y/N] ");

                io::stdout().flush().map_err(|err| err.to_string())?;

                match sc.next_line().map_err(|err| match err {
                    ScannerError::IOError(err) => err.to_string(),
                    _ => unreachable!()
                })? {
                    Some(token) => {
                        match Boolean::from_string(token) {
                            Ok(token) => {
                                if token.get_bool() {
                                    break;
                                } else {
                                    return Ok(0);
                                }
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    }
                    None => {
                        return Ok(0);
                    }
                }
            }
        }
    } else {
        fs::create_dir_all(&config.output).map_err(|err| err.to_string())?;
    }

    let mut tera = Tera::default();

    tera.add_raw_template("browser-config", include_str!("resources/browser-config.xml")).map_err(|err| err.to_string())?;

    let (ident, mw) = {
        let input = ImageResource::Path(input_str.to_string());

        let mut output = Some(None);

        let ident = identify(&mut output, &input).map_err(|err| err.to_string())?;

        (ident, output.unwrap().unwrap())
    };

    let sharpen = if config.sharpen {
        match ident.format.as_str() {
            "MVG" | "SVG" => false,
            _ => true
        }
    } else {
        false
    };

    let mw_input = ImageResource::MagickWand(mw);

    {
        // web_app_manifest
        let src_192 = format!("{}favicon-192.png", path_prefix);
        let src_512 = format!("{}favicon-512.png", path_prefix);

        let content = json!(
                {
                    "name": config.app_name,
                    "short_name": config.app_short_name,
                    "icons": [
                        {
                            "src": src_192,
                            "sizes": "192x192",
                            "type": "image/png"
                        },
                        {
                            "src": src_512,
                            "sizes": "512x512",
                            "type": "image/png"
                        }
                    ],
                    "theme_color": &config.android_background_color,
                    "background_color": config.android_background_color,
                    "display": "standalone"
                }
            );

        let content = serde_json::to_string(&content).unwrap();

        fs::write(web_app_manifest, content).map_err(|err| err.to_string())?;
    }

    {
        // browser_config
        let mut context = Context::new();

        context.insert("path_prefix", path_prefix);
        context.insert("background_color", &config.windows_background_color);

        let content = tera.render("browser-config", context).map_err(|err| err.to_string())?;

        fs::write(browser_config, content).map_err(|err| err.to_string())?;
    }

    {
        // svg_monochrome
        let mut pgm_config = PGMConfig::new();

        pgm_config.background_color = Some(ColorName::White);

        let mut output = ImageResource::Data(Vec::new());

        to_pgm(&mut output, &mw_input, &pgm_config).map_err(|err| err.to_string())?;

        let pgm_data = output.into_vec().unwrap();

        let threshold_string = format!("{:.3}", config.threshold);

        let rtn = execute_one_stdin(&vec![potrace, "-s", "-k", threshold_string.as_str(), "-", "-o", FILE_SVG_MONOCHROME], output_str, pgm_data)?;

        if rtn != 0 {
            return Err(format!("Fail to build `{}`.", svg_monochrome.to_string_lossy()));
        }
    }

    {
        // ico
        let mut ico_config = ICOConfig::new();

        if !sharpen {
            ico_config.sharpen = 0f64;
        }

        for &size in ICO_SIZE.iter() {
            ico_config.size.push((size, size));
        }

        let mut output = ImageResource::from_path(ico);

        to_ico(&mut output, &mw_input, &ico_config).map_err(|err| err.to_string())?;
    }

    {
        // png_vec
        for (i, png) in png_vec.iter().enumerate() {
            let size = PNG_SIZE[i];

            let mut png_config = PNGConfig::new();
            png_config.shrink_only = false;
            png_config.width = size;
            png_config.height = size;

            if !sharpen {
                png_config.sharpen = 0f64;
            }

            let mut output = ImageResource::from_path(png);

            to_png(&mut output, &mw_input, &png_config).map_err(|err| err.to_string())?;
        }
    }

    {
        // png_ios_background
        let mw = mw_input.into_magick_wand().unwrap();

        let mut pw = PixelWand::new();
        pw.set_color(config.ios_background_color.as_str())?;
        mw.set_image_background_color(&pw)?;
        mw.set_image_alpha_channel(bindings::AlphaChannelOption_RemoveAlphaChannel)?;

        let mut png_config = PNGConfig::new();
        png_config.shrink_only = false;
        png_config.width = 180;
        png_config.height = 180;

        if !sharpen {
            png_config.sharpen = 0f64;
        }

        let mw_input = ImageResource::MagickWand(mw);

        let mut output = ImageResource::from_path(png_ios_background);

        to_png(&mut output, &mw_input, &png_config).map_err(|err| err.to_string())?;
    }

    tera.add_raw_template("html-head", include_str!("resources/favicon.html")).map_err(|err| err.to_string())?;

    let mut context = Context::new();

    context.insert("path_prefix", path_prefix);
    context.insert("android_background_color", &config.android_background_color);
    context.insert("windows_background_color", &config.windows_background_color);
    context.insert("web_app_manifest", FILE_WEB_APP_MANIFEST);
    context.insert("png_ios_background", FILE_PNG_IOS_BACKGROUND);

    let content = tera.render("html-head", context).map_err(|err| err.to_string())?;

    println!("{}", content);

    Ok(0)
}