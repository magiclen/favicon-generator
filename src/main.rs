use std::{
    env,
    error::Error,
    fmt::Write as FmtWrite,
    fs,
    io::{self, ErrorKind, Write},
    path::Path,
};

use clap::{Arg, Command};
use concat_with::concat_line;
use favicon_generator::*;
use scanner_rust::{generic_array::typenum::U8, Scanner};
use serde_json::json;
use terminal_size::terminal_size;
use validators::prelude::*;

const APP_NAME: &str = "Favicon Generator";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const DEFAULT_PATH_PREFIX: &str = "/";

const FILE_WEB_APP_MANIFEST: &str = "manifest.json";
const FILE_FAVICON: &str = "favicon.ico";

const ICO_SIZE: &[u16] = &[48, 32, 16];
const PNG_SIZES_FOR_ICON: &[u16] = &[196, 160, 95, 64, 32, 16];
const PNG_SIZES_FOR_APPLE_TOUCH_ICON: &[u16] = &[180, 152, 144, 120, 114, 76, 72, 60, 57];

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new(APP_NAME)
        .term_width(terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about(concat!("It helps you generate favicons with different formats and sizes.\n\nEXAMPLES:\n", concat_line!(prefix "favicon-generator ",
                "/path/to/image /path/to/folder     # Uses /path/to/image to generate favicons into /path/to/folder",
            )))
        .arg(Arg::new("INPUT_PATH")
            .required(true)
            .help("Assign an image for generating favicons. It should be a path of a file")
            .takes_value(true)
            .display_order(0)
        )
        .arg(Arg::new("OUTPUT_PATH")
            .required(true)
            .help("Assign a destination of your generated files. It should be a path of a directory")
            .takes_value(true)
            .display_order(1)
        )
        .arg(Arg::new("OVERWRITE")
            .long("overwrite")
            .short('y')
            .help("Overwrite exiting files without asking")
            .display_order(2)
        )
        .arg(Arg::new("PATH_PREFIX")
            .long("path-prefix")
            .help("Specify the path prefix of your favicon files")
            .takes_value(true)
            .default_value(DEFAULT_PATH_PREFIX)
            .display_order(3)
        )
        .arg(Arg::new("NO_SHARPEN")
            .long("no-sharpen")
            .help("Disable the automatic sharpening")
            .display_order(4)
        )
        .arg(Arg::new("APP_NAME")
            .long("app-name")
            .help("Assign a name for your web app")
            .takes_value(true)
            .default_value("App")
            .display_order(10)
        )
        .arg(Arg::new("APP_SHORT_NAME")
            .long("app-short-name")
            .help("Assign a short name for your web app")
            .takes_value(true)
            .display_order(11)
        )
        .after_help("Enjoy it! https://magiclen.org")
        .get_matches();

    let input = matches.value_of("INPUT_PATH").unwrap();
    let output = matches.value_of("OUTPUT_PATH").unwrap();
    let path_prefix = matches.value_of("PATH_PREFIX").unwrap();
    let overwrite = matches.is_present("OVERWRITE");
    let sharpen = !matches.is_present("NO_SHARPEN");
    let app_name = matches.value_of("APP_NAME").unwrap();
    let app_short_name = matches.value_of("APP_SHORT_NAME");

    let output_path = Path::new(output);

    let web_app_manifest = output_path.join(FILE_WEB_APP_MANIFEST);
    let ico = output_path.join(FILE_FAVICON);

    let png_sizes = {
        let mut v = PNG_SIZES_FOR_ICON.to_vec();

        v.extend_from_slice(PNG_SIZES_FOR_APPLE_TOUCH_ICON);

        v.sort();

        v
    };

    let png_vec = {
        let mut v = Vec::with_capacity(png_sizes.len());

        for size in png_sizes.iter() {
            v.push(output_path.join(format!("favicon-{size}.png")));
        }

        v
    };

    if output_path.exists() {
        let need_overwrite = {
            let mut path_vec = Vec::with_capacity(2 + png_vec.len());

            path_vec.push(&ico);
            path_vec.push(&web_app_manifest);

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

                            break;
                        } else {
                            return Err(
                                format!("`{}` is not a file.", path.to_string_lossy()).into()
                            );
                        }
                    },
                    Err(ref err) if err.kind() == ErrorKind::NotFound => {
                        // do nothing
                    },
                    Err(err) => {
                        return Err(err.into());
                    },
                }
            }

            need_overwrite
        };

        if need_overwrite && !overwrite {
            let mut sc: Scanner<_, U8> = Scanner::new2(io::stdin());

            loop {
                print!("Overwrite files? [Y/N] ");

                io::stdout().flush()?;

                match sc.next_line()? {
                    Some(token) => match Boolean::parse_string(token) {
                        Ok(token) => {
                            if token.get_bool() {
                                break;
                            } else {
                                return Ok(());
                            }
                        },
                        Err(_) => {
                            continue;
                        },
                    },
                    None => {
                        return Ok(());
                    },
                }
            }
        }
    } else {
        fs::create_dir_all(output_path)?;
    }

    let input = image_convert::ImageResource::Data(match fs::read(input) {
        Ok(data) => data,
        Err(ref err) if err.kind() == ErrorKind::NotFound => {
            return Err(format!("`{input}` is not a file.").into());
        },
        Err(err) => return Err(err.into()),
    });

    let mut tera = tera::Tera::default();

    tera.add_raw_template("browser-config", include_str!("resources/browser-config.xml"))?;

    {
        // web_app_manifest
        let mut icons = Vec::with_capacity(PNG_SIZES_FOR_ICON.len());

        for size in PNG_SIZES_FOR_ICON {
            let src = format!("{path_prefix}favicon-{size}.png");
            let sizes = format!("{size}x{size}");

            icons.push(json!({
                "src": src,
                "sizes": sizes,
                "type": "image/png",
            }));
        }

        let mut content = json!(
            {
                "name": app_name,
                "icons": icons,
            }
        );

        if let Some(app_short_name) = app_short_name {
            content.as_object_mut().unwrap().insert("short_name".into(), app_short_name.into());
        }

        let content = serde_json::to_string(&content).unwrap();

        fs::write(web_app_manifest, content)?;
    }

    let (input, vector) = {
        let mut pgm_config = image_convert::PGMConfig::new();

        pgm_config.background_color = Some(image_convert::ColorName::White);
        pgm_config.crop = Some(image_convert::Crop::Center(1f64, 1f64));

        let (mw, vector) = image_convert::fetch_magic_wand(&input, &pgm_config)?;

        let mw_input = image_convert::ImageResource::MagickWand(mw);

        (mw_input, vector)
    };

    let sharpen = if vector { false } else { sharpen };

    {
        // ico
        let mut ico_config = image_convert::ICOConfig::new();

        if !sharpen {
            ico_config.sharpen = 0f64;
        }

        for size in ICO_SIZE.iter().copied() {
            ico_config.size.push((size, size));
        }

        let mut output = image_convert::ImageResource::from_path(ico);

        image_convert::to_ico(&mut output, &input, &ico_config)?;
    }

    {
        // png_vec
        for (i, png) in png_vec.iter().enumerate() {
            let size = png_sizes[i];

            let mut png_config = image_convert::PNGConfig::new();
            png_config.shrink_only = false;
            png_config.width = size;
            png_config.height = size;

            if !sharpen {
                png_config.sharpen = 0f64;
            }

            let mut output = image_convert::ImageResource::from_path(png);

            image_convert::to_png(&mut output, &input, &png_config)?;
        }
    }

    let ico_sizes_concat = {
        let mut s = String::new();

        for size in ICO_SIZE {
            s.write_fmt(format_args!("{size}x{size} ")).unwrap();
        }

        s.truncate(s.len() - 1);

        s
    };

    tera.add_raw_template("html-head", include_str!("resources/favicon.html"))?;

    let mut context = tera::Context::new();

    context
        .insert("path_prefix", html_escape::encode_double_quoted_attribute(path_prefix).as_ref());
    context.insert("web_app_manifest", FILE_WEB_APP_MANIFEST);
    context.insert("apple_touch_icon_sizes", PNG_SIZES_FOR_APPLE_TOUCH_ICON);
    context.insert("icon_sizes", PNG_SIZES_FOR_ICON);
    context.insert("ico_sizes_concat", &ico_sizes_concat);

    let content = tera.render("html-head", &context)?;

    println!("{content}");

    Ok(())
}
