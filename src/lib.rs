//! # Favicon Generator
//! It helps you generate favicons for different platforms.

extern crate clap;
extern crate image_convert;

use std::env;
use std::path::Path;
use std::fs;
use std::thread;
use std::sync::mpsc;

use clap::{App, Arg};

// TODO -----Config START-----

const APP_NAME: &str = "Favicon Generator";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const PNG_SIZE: [u16; 18] = [16, 32, 36, 48, 57, 60, 70, 72, 76, 96, 114, 120, 144, 150, 152, 180, 192, 310];

#[derive(Debug)]
pub struct Config {
    pub input: String,
    pub output: String,
}

impl Config {
    pub fn from_cli() -> Result<Config, String> {
        let arg0 = env::args().next().unwrap();
        let arg0 = Path::new(&arg0).file_stem().unwrap().to_str().unwrap();

        let examples = vec![
            "-i /path/to/image -o /path/to/folder     # Use /path/to/image to generate favicons into /path/to/folder",
        ];

        let matches = App::new(APP_NAME)
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
                .long("input")
                .short("i")
                .help("Assigns an image for generating favicons. It should be a path of a file.")
                .takes_value(true)
            )
            .arg(Arg::with_name("OUTPUT_PATH")
                .required(true)
                .long("output")
                .short("o")
                .help("Assigns a destination of your generated files. It should be a path of a directory.")
                .takes_value(true)
            )
            .after_help("Enjoy it! https://magiclen.org")
            .get_matches();

        let input = matches.value_of("INPUT_PATH").unwrap();

        let input_obj = match Path::new(&input).canonicalize() {
            Ok(p) => {
                if !p.is_file() {
                    return Err(String::from("INPUT_PATH is incorrect."));
                }
                p
            }
            Err(_) => return Err(String::from("INPUT_PATH is incorrect."))
        };

        let input = String::from(input_obj.to_str().unwrap());

        let output = matches.value_of("OUTPUT_PATH").unwrap();

        let output_obj = Path::new(&output);

        if output_obj.exists() {
            if output_obj.is_dir() {
                if let Err(_) = fs::remove_dir(output_obj) {
                    return Err(String::from("OUTPUT_PATH is not empty."));
                }
            } else {
                return Err(String::from("OUTPUT_PATH exists."));
            }
        }

        if let Err(_) = fs::create_dir_all(output_obj) {
            return Err(String::from("OUTPUT_PATH is incorrect."));
        }

        let output_obj = output_obj.canonicalize().unwrap();

        let output = String::from(output_obj.to_str().unwrap());

        Ok(Config {
            input,
            output,
        })
    }
}

// TODO -----Config END-----

pub fn run(config: Config) -> Result<i32, String> {
    let output_path = Path::new(&config.output);

    let favicon_path = Path::join(&output_path, Path::new("favicon.ico"));

    let favicons_path = Path::join(&output_path, Path::new("favicons"));

    if let Err(_) = fs::create_dir_all(&favicons_path) {
        return Err(String::from("Cannot create favicons folder."));
    }

    let mut ico_config = image_convert::ICOConfig::new();

    ico_config.size.push((48, 48));
    ico_config.size.push((32, 32));
    ico_config.size.push((16, 16));

    let image_source = image_convert::ImageResource::Path(&config.input);
    let mut image_destination = image_convert::ImageResource::Path(favicon_path.to_str().unwrap());

    if let Err(e) = image_convert::to_ico(&mut image_destination, &image_source, &ico_config) {
        return Err(e.to_string());
    }

    let (tx_1, rx) = mpsc::channel();

    for &png_size in PNG_SIZE.iter() {
        let input = config.input.clone();
        let favicons_path = favicons_path.clone();

        let tx_2 = tx_1.clone();

        thread::spawn(move || {
            let file_name = format!("favicon-{}.png", png_size);
            let png_path = Path::join(&favicons_path, Path::new(&file_name));
            let image_source = image_convert::ImageResource::Path(&input);
            let mut image_destination = image_convert::ImageResource::Path(png_path.to_str().unwrap());
            let mut png_config = image_convert::PNGConfig::new();
            png_config.shrink_only = false;
            png_config.width = png_size;
            png_config.height = png_size;

            image_convert::to_png(&mut image_destination, &image_source, &png_config).unwrap();
            tx_2.send(0).unwrap();
        });
    }

    let browserconfig_path = Path::join(&favicons_path, Path::new("browserconfig.xml"));

    fs::write(&browserconfig_path, include_str!("resources/browserconfig.xml")).unwrap();

    let manifest_path = Path::join(&favicons_path, Path::new("manifest.json"));

    fs::write(&manifest_path, include_str!("resources/manifest.json")).unwrap();

    let html_path = Path::join(&output_path, Path::new("favicon.html"));

    fs::write(&html_path, include_str!("resources/favicon.html")).unwrap();

    for _ in PNG_SIZE.iter() {
        rx.recv().unwrap();
    }

    Ok(0)
}