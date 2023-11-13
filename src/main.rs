mod cli;

use std::{fmt::Write as FmtWrite, fs, io, io::Write};

use anyhow::{anyhow, Context};
use cli::*;
use scanner_rust::{generic_array::typenum::U8, Scanner};
use serde_json::json;

const FILE_WEB_APP_MANIFEST: &str = "manifest.json";
const FILE_FAVICON: &str = "favicon.ico";

const ICO_SIZE: &[u16] = &[48, 32, 16];
const PNG_SIZES_FOR_ICON: &[u16] = &[196, 160, 95, 64, 32, 16];
const PNG_SIZES_FOR_APPLE_TOUCH_ICON: &[u16] = &[180, 152, 144, 120, 114, 76, 72, 60, 57];

fn main() -> anyhow::Result<()> {
    let args = get_args();

    let web_app_manifest = args.output_path.join(FILE_WEB_APP_MANIFEST);
    let ico = args.output_path.join(FILE_FAVICON);

    let png_sizes = {
        let mut v = PNG_SIZES_FOR_ICON.to_vec();

        v.extend_from_slice(PNG_SIZES_FOR_APPLE_TOUCH_ICON);

        v.sort();

        v
    };

    let png_vec = {
        let mut v = Vec::with_capacity(png_sizes.len());

        for size in png_sizes.iter() {
            v.push(args.output_path.join(format!("favicon-{size}.png")));
        }

        v
    };

    match args.output_path.metadata() {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return Err(anyhow!("{:?} is not a directory.", args.output_path));
            }

            let need_overwrite = {
                let mut path_vec = Vec::with_capacity(2 + png_vec.len());

                path_vec.push(&ico);
                path_vec.push(&web_app_manifest);

                for png in png_vec.iter() {
                    path_vec.push(png);
                }

                let mut need_overwrite = false;

                for path in path_vec {
                    match path.metadata() {
                        Ok(metadata) => {
                            if metadata.is_dir() {
                                return Err(anyhow!("{path:?} is a directory."));
                            }

                            need_overwrite = true;
                        },
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {
                            // do nothing
                        },
                        Err(error) => {
                            return Err(error).with_context(|| anyhow!("{path:?}"));
                        },
                    }
                }

                need_overwrite
            };

            if need_overwrite && !args.overwrite {
                let mut sc: Scanner<_, U8> = Scanner::new2(io::stdin());

                loop {
                    print!("Overwrite files? [Y/N] ");
                    io::stdout().flush().with_context(|| "stdout")?;

                    match sc.next_line().with_context(|| "stdin")? {
                        Some(token) => match token.to_ascii_uppercase().as_str() {
                            "Y" => {
                                break;
                            },
                            "N" => {
                                return Ok(());
                            },
                            _ => {
                                continue;
                            },
                        },
                        None => {
                            return Ok(());
                        },
                    }
                }
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(args.output_path.as_path())
                .with_context(|| anyhow!("{:?}", args.output_path))?;
        },
        Err(error) => {
            return Err(error).with_context(|| anyhow!("{:?}", args.output_path));
        },
    }

    let input = image_convert::ImageResource::Data(
        fs::read(args.input_path.as_path()).with_context(|| anyhow!("{:?}", args.input_path))?,
    );

    let mut tera = tera::Tera::default();

    tera.add_raw_template("browser-config", include_str!("resources/browser-config.xml")).unwrap();

    {
        // web_app_manifest
        let mut icons = Vec::with_capacity(PNG_SIZES_FOR_ICON.len());

        for size in PNG_SIZES_FOR_ICON {
            let src = format!("{path_prefix}favicon-{size}.png", path_prefix = args.path_prefix);
            let sizes = format!("{size}x{size}");

            icons.push(json!({
                "src": src,
                "sizes": sizes,
                "type": "image/png",
            }));
        }

        let mut content = json!(
            {
                "name": args.app_name,
                "icons": icons,
            }
        );

        if let Some(app_short_name) = args.app_short_name {
            content.as_object_mut().unwrap().insert("short_name".into(), app_short_name.into());
        }

        let content = serde_json::to_string(&content).unwrap();

        fs::write(web_app_manifest.as_path(), content)
            .with_context(|| anyhow!("{web_app_manifest:?}"))?;
    }

    let (input, vector) = {
        let mut pgm_config = image_convert::PGMConfig::new();

        pgm_config.background_color = Some(image_convert::ColorName::White);
        pgm_config.crop = Some(image_convert::Crop::Center(1f64, 1f64));

        let (mw, vector) = image_convert::fetch_magic_wand(&input, &pgm_config)
            .with_context(|| anyhow!("fetch_magic_wand {:?}", args.input_path))?;

        let mw_input = image_convert::ImageResource::MagickWand(mw);

        (mw_input, vector)
    };

    let sharpen = if vector { false } else { !args.no_sharpen };

    {
        // ico
        let mut ico_config = image_convert::ICOConfig::new();

        if !sharpen {
            ico_config.sharpen = 0f64;
        }

        for size in ICO_SIZE.iter().copied() {
            ico_config.size.push((size, size));
        }

        let mut output = image_convert::ImageResource::from_path(ico.as_path());

        image_convert::to_ico(&mut output, &input, &ico_config)
            .with_context(|| anyhow!("to_ico {ico:?}"))?;
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

            let mut output = image_convert::ImageResource::from_path(png.as_path());

            image_convert::to_png(&mut output, &input, &png_config)
                .with_context(|| anyhow!("to_ico {png:?}"))?;
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

    tera.add_raw_template("html-head", include_str!("resources/favicon.html")).unwrap();

    let mut context = tera::Context::new();

    context.insert(
        "path_prefix",
        html_escape::encode_double_quoted_attribute(args.path_prefix.as_str()).as_ref(),
    );
    context.insert("web_app_manifest", FILE_WEB_APP_MANIFEST);
    context.insert("apple_touch_icon_sizes", PNG_SIZES_FOR_APPLE_TOUCH_ICON);
    context.insert("icon_sizes", PNG_SIZES_FOR_ICON);
    context.insert("ico_sizes_concat", &ico_sizes_concat);

    let content = tera.render("html-head", &context).unwrap();

    println!("{content}");

    Ok(())
}
