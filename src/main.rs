#[macro_use]
extern crate concat_with;
extern crate clap;
extern crate terminal_size;

#[macro_use]
extern crate serde_json;

extern crate execute;
extern crate image_convert;
extern crate scanner_rust;
extern crate slash_formatter;
extern crate tera;
extern crate validators;

extern crate favicon_generator;

use std::env;
use std::fs;
use std::io::{self, ErrorKind, Write};
use std::path::Path;
use std::process::{self, Command};

use clap::{App, Arg};
use terminal_size::terminal_size;

use scanner_rust::generic_array::typenum::U8;
use scanner_rust::Scanner;

use image_convert::magick_rust;

use validators::boolean::Boolean;

use execute::Execute;

use favicon_generator::*;

const APP_NAME: &str = "Favicon Generator";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const DEFAULT_POTRACE_PATH: &str = "potrace";
const DEFAULT_PATH_PREFIX: &str = "/";
const DEFAULT_THRESHOLD: &str = "0.5";
const DEFAULT_BACKGROUND_COLOR: &str = "#ffffff";
const DEFAULT_SAFARI_BACKGROUND_COLOR: &str = "#000000";

const FILE_WEB_APP_MANIFEST: &str = "web-app.manifest";
const FILE_BROWSER_CONFIG: &str = "browser-config.xml";
const FILE_SVG_MONOCHROME: &str = "favicon-monochrome.svg";
const FILE_PNG_IOS_BACKGROUND: &str = "favicon-180-i.png";
const FILE_FAVICON: &str = "favicon.ico";

const ICO_SIZE: [u16; 3] = [48, 32, 16];
const PNG_SIZE: [u16; 4] = [512, 192, 32, 16];
const MSTILE_SIZE: [(u16, u16, u16, u16); 3] =
    [(310, 558, 256, 151), (150, 270, 128, 48), (70, 128, 96, 16)];

fn main() -> Result<(), String> {
    let matches = App::new(APP_NAME)
        .set_term_width(terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about(concat!("It helps you generate favicons with different formats and sizes.\n\nEXAMPLES:\n", concat_line!(prefix "favicon-generator ",
                "/path/to/image /path/to/folder     # Uses /path/to/image to generate favicons into /path/to/folder",
            )))
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
        .arg(Arg::with_name("SAFARI_BACKGROUND_COLOR").value_name("HEX_COLOR")
            .long("safari-background-color").visible_alias("safari-background")
            .help("Assigns a background color for Safari")
            .takes_value(true)
            .default_value(DEFAULT_SAFARI_BACKGROUND_COLOR)
            .display_order(15)
        )
        .arg(Arg::with_name("WINDOWS_BACKGROUND_COLOR").value_name("HEX_COLOR")
            .long("windows-background-color").visible_alias("windows-background")
            .help("Assigns a background color for Windows devices")
            .takes_value(true)
            .default_value(DEFAULT_BACKGROUND_COLOR)
            .display_order(16)
        )
        .after_help("Enjoy it! https://magiclen.org")
        .get_matches();

    let (
        android_background_color,
        ios_background_color,
        safari_background_color,
        windows_background_color,
    ) = match matches.value_of("BACKGROUND_COLOR") {
        Some(background_color) => {
            let background_color =
                HexColor::from_str(background_color).map_err(|err| err.to_string())?;

            (
                background_color.clone(),
                background_color.clone(),
                background_color.clone(),
                background_color,
            )
        }
        None => {
            (
                HexColor::from_str(matches.value_of("ANDROID_BACKGROUND_COLOR").unwrap())
                    .map_err(|err| err.to_string())?,
                HexColor::from_str(matches.value_of("IOS_BACKGROUND_COLOR").unwrap())
                    .map_err(|err| err.to_string())?,
                HexColor::from_str(matches.value_of("SAFARI_BACKGROUND_COLOR").unwrap())
                    .map_err(|err| err.to_string())?,
                HexColor::from_str(matches.value_of("WINDOWS_BACKGROUND_COLOR").unwrap())
                    .map_err(|err| err.to_string())?,
            )
        }
    };

    let potrace = matches.value_of("POTRACE_PATH").unwrap();
    let input = matches.value_of("INPUT_PATH").unwrap();
    let output = matches.value_of("OUTPUT_PATH").unwrap();
    let path_prefix = matches.value_of("PATH_PREFIX").unwrap();
    let overwrite = matches.is_present("OVERWRITE");
    let threshold = Threshold::from_str(matches.value_of("THRESHOLD").unwrap())
        .map_err(|err| err.to_string())?;
    let sharpen = !matches.is_present("NO_SHARPEN");
    let app_name = matches.value_of("APP_NAME").unwrap_or("");
    let app_short_name = matches.value_of("APP_SHORT_NAME").unwrap_or("");

    if Command::new(potrace).args(&["-v"]).execute_check_exit_status_code(0).is_err() {
        return Err(format!("Cannot execute `{}`.", potrace));
    }

    let output_path = Path::new(output);

    let web_app_manifest = output_path.join(FILE_WEB_APP_MANIFEST);
    let browser_config = output_path.join(FILE_BROWSER_CONFIG);
    let svg_monochrome = output_path.join(FILE_SVG_MONOCHROME);
    let png_ios_background = output_path.join(FILE_PNG_IOS_BACKGROUND);
    let ico = output_path.join(FILE_FAVICON);

    let png_vec = {
        let mut v = Vec::with_capacity(PNG_SIZE.len());

        for &size in PNG_SIZE.iter() {
            v.push(output_path.join(format!("favicon-{}.png", size)));
        }

        v
    };

    let mstile_vec = {
        let mut v = Vec::with_capacity(MSTILE_SIZE.len());

        for &size in MSTILE_SIZE.iter() {
            v.push(output_path.join(format!("mstile-{}.png", size.0)));
        }

        v
    };

    if output_path.exists() {
        let need_overwrite = {
            let mut path_vec = Vec::with_capacity(5 + PNG_SIZE.len() + MSTILE_SIZE.len());

            path_vec.extend_from_slice(&[
                &web_app_manifest,
                &browser_config,
                &svg_monochrome,
                &png_ios_background,
                &ico,
            ]);

            for png in png_vec.iter() {
                path_vec.push(png);
            }

            for mstile in mstile_vec.iter() {
                path_vec.push(mstile);
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

        if need_overwrite && !overwrite {
            let mut sc: Scanner<_, U8> = Scanner::new2(io::stdin());

            loop {
                print!("Overwrite files? [Y/N] ");

                io::stdout().flush().map_err(|err| err.to_string())?;

                match sc.next_line().map_err(|err| err.to_string())? {
                    Some(token) => {
                        match Boolean::from_string(token) {
                            Ok(token) => {
                                if token.get_bool() {
                                    break;
                                } else {
                                    return Ok(());
                                }
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    }
                    None => {
                        return Ok(());
                    }
                }
            }
        }
    } else {
        fs::create_dir_all(output_path).map_err(|err| err.to_string())?;
    }

    let input = image_convert::ImageResource::Data(match fs::read(input) {
        Ok(data) => data,
        Err(ref err) if err.kind() == ErrorKind::NotFound => {
            return Err(format!("`{}` is not a file.", input));
        }
        Err(err) => return Err(err.to_string()),
    });

    let mut tera = tera::Tera::default();

    tera.add_raw_template("browser-config", include_str!("resources/browser-config.xml"))
        .map_err(|err| err.to_string())?;

    {
        // web_app_manifest
        let src_192 = format!("{}favicon-192.png", path_prefix);
        let src_512 = format!("{}favicon-512.png", path_prefix);

        let content = json!(
            {
                "name": app_name,
                "short_name": app_short_name,
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
                "theme_color": android_background_color,
                "background_color": android_background_color,
                "display": "standalone"
            }
        );

        let content = serde_json::to_string(&content).unwrap();

        fs::write(web_app_manifest, content).map_err(|err| err.to_string())?;
    }

    {
        // browser_config
        let mut context = tera::Context::new();

        context.insert("path_prefix", path_prefix);
        context.insert("background_color", &windows_background_color);
        context.insert("mstile_size", &MSTILE_SIZE);

        let content = tera.render("browser-config", &context).map_err(|err| err.to_string())?;

        fs::write(browser_config, content).map_err(|err| err.to_string())?;
    }

    let (input, vector) = {
        // svg_monochrome
        let mut pgm_config = image_convert::PGMConfig::new();

        pgm_config.background_color = Some(image_convert::ColorName::White);

        let (mw, vector) =
            image_convert::fetch_magic_wand(&input, &pgm_config).map_err(|err| err.to_string())?;

        let mw_input = image_convert::ImageResource::MagickWand(mw);

        let potrace_output_path =
            slash_formatter::concat_with_file_separator(output, FILE_SVG_MONOCHROME);

        let mut output = image_convert::ImageResource::Data(Vec::new());

        image_convert::to_pgm(&mut output, &mw_input, &pgm_config)
            .map_err(|err| err.to_string())?;

        let pgm_data = output.into_vec().unwrap();

        let threshold_string = format!("{:.3}", threshold);

        let rtn = Command::new(potrace)
            .args(&["-s", "-k", threshold_string.as_str(), "-", "-o", potrace_output_path.as_str()])
            .execute_input(&pgm_data)
            .map_err(|err| err.to_string())?;

        match rtn {
            Some(code) => {
                if code != 0 {
                    return Err(format!("Fail to build `{}`.", svg_monochrome.to_string_lossy()));
                }
            }
            None => {
                process::exit(1);
            }
        }

        (mw_input, vector)
    };

    let sharpen = if vector {
        false
    } else {
        sharpen
    };

    {
        // ico
        let mut ico_config = image_convert::ICOConfig::new();

        if !sharpen {
            ico_config.sharpen = 0f64;
        }

        for &size in ICO_SIZE.iter() {
            ico_config.size.push((size, size));
        }

        let mut output = image_convert::ImageResource::from_path(ico);

        image_convert::to_ico(&mut output, &input, &ico_config).map_err(|err| err.to_string())?;
    }

    {
        // png_vec
        for (i, png) in png_vec.iter().enumerate() {
            let size = PNG_SIZE[i];

            let mut png_config = image_convert::PNGConfig::new();
            png_config.shrink_only = false;
            png_config.width = size;
            png_config.height = size;

            if !sharpen {
                png_config.sharpen = 0f64;
            }

            let mut output = image_convert::ImageResource::from_path(png);

            image_convert::to_png(&mut output, &input, &png_config)
                .map_err(|err| err.to_string())?;
        }
    }

    {
        // mstile_vec
        for (i, mstile) in mstile_vec.iter().enumerate() {
            let size = MSTILE_SIZE[i];

            let mut png_config = image_convert::PNGConfig::new();
            png_config.shrink_only = false;
            png_config.width = size.2;
            png_config.height = size.2;

            if !sharpen {
                png_config.sharpen = 0f64;
            }

            let mut output = image_convert::ImageResource::Data(Vec::new());

            image_convert::to_png(&mut output, &input, &png_config)
                .map_err(|err| err.to_string())?;

            let mw_i = magick_rust::MagickWand::new();

            mw_i.read_image_blob(output.into_vec().unwrap())?;

            let mut mw = magick_rust::MagickWand::new();

            mw.set_format("PNG32")?;

            let mut pw = magick_rust::PixelWand::new();
            pw.set_color("none")?;
            mw.new_image(size.1 as usize, size.1 as usize, &pw)?;

            mw.compose_images(
                &mw_i,
                magick_rust::bindings::CompositeOperator_OverCompositeOp,
                false,
                ((size.1 - size.2) / 2) as isize,
                size.3 as isize,
            )?;

            let input = image_convert::ImageResource::MagickWand(mw);

            png_config.width = 0;
            png_config.height = 0;
            png_config.sharpen = 0f64;

            output = image_convert::ImageResource::from_path(mstile);

            image_convert::to_png(&mut output, &input, &png_config)
                .map_err(|err| err.to_string())?;
        }
    }

    {
        // png_ios_background
        let mut png_config = image_convert::PNGConfig::new();
        png_config.shrink_only = false;
        png_config.width = 180;
        png_config.height = 180;

        let mw = if vector {
            let (mw, vector) = image_convert::fetch_magic_wand(&input, &png_config)
                .map_err(|err| err.to_string())?;
            if !vector {
                return Err("The input image may not be a correct vector.".to_string());
            }

            mw
        } else {
            input.into_magick_wand().unwrap()
        };

        let mut pw = magick_rust::PixelWand::new();
        pw.set_color(ios_background_color.as_str())?;
        mw.set_image_background_color(&pw)?;
        mw.set_image_alpha_channel(magick_rust::bindings::AlphaChannelOption_RemoveAlphaChannel)?;

        if !sharpen {
            png_config.sharpen = 0f64;
        }

        let input = image_convert::ImageResource::MagickWand(mw);

        let mut output = image_convert::ImageResource::from_path(png_ios_background);

        image_convert::to_png(&mut output, &input, &png_config).map_err(|err| err.to_string())?;
    }

    tera.add_raw_template("html-head", include_str!("resources/favicon.html"))
        .map_err(|err| err.to_string())?;

    let mut context = tera::Context::new();

    context.insert("path_prefix", path_prefix);
    context.insert("android_background_color", &android_background_color);
    context.insert("windows_background_color", &windows_background_color);
    context.insert("safari_background_color", &safari_background_color);
    context.insert("web_app_manifest", FILE_WEB_APP_MANIFEST);
    context.insert("png_ios_background", FILE_PNG_IOS_BACKGROUND);
    context.insert("svg_monochrome", FILE_SVG_MONOCHROME);
    context.insert("browser_config", FILE_BROWSER_CONFIG);
    context.insert("png_size", &PNG_SIZE);

    let content = tera.render("html-head", &context).map_err(|err| err.to_string())?;

    println!("{}", content);

    Ok(())
}
