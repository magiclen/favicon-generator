mod cli;
mod favicon;
mod html;
mod manifest;

use std::{fs, io, io::Write};

use anyhow::{Context, anyhow};
use cli::*;
use favicon::OutputFiles;
use image_convert::ImageResource;
use manifest::WebAppManifest;

fn main() -> anyhow::Result<()> {
    let args = get_args();

    // the input image decides whether an SVG icon is written, so it is read before the output files are listed
    let input = ImageResource::Data(
        fs::read(args.input_path.as_path()).with_context(|| anyhow!("{:?}", args.input_path))?,
    );

    let identify =
        image_convert::identify_ping(&input).with_context(|| anyhow!("{:?}", args.input_path))?;

    let vector = favicon::is_vector(&identify);

    let output_files = OutputFiles::new(args.output_path.as_path(), vector);

    if !prepare_output_directory(&args, &output_files)? {
        return Ok(());
    }

    let sharpen = !vector && !args.no_sharpen;

    if let Some(svg) = output_files.svg.as_deref() {
        favicon::generate_svg(svg, input.as_u8_slice().unwrap())?;
    }

    let source = favicon::prepare_source(input, &identify)
        .with_context(|| anyhow!("{:?}", args.input_path))?;

    favicon::generate_ico(output_files.ico.as_path(), &source, sharpen)?;

    favicon::generate_apple_touch_icon(
        output_files.apple_touch_icon.as_path(),
        &source,
        args.background_color.as_str(),
        sharpen,
    )?;

    for (size, png) in output_files.manifest_icons.iter() {
        favicon::generate_png(png.as_path(), &source, *size, sharpen)?;
    }

    favicon::generate_maskable_icon(
        output_files.maskable_icon.as_path(),
        &source,
        args.background_color.as_str(),
        sharpen,
    )?;

    let web_app_manifest = output_files.web_app_manifest.as_path();

    fs::write(web_app_manifest, WebAppManifest::new(&args).to_json())
        .with_context(|| anyhow!("{web_app_manifest:?}"))?;

    println!("{}", html::build_head(&args, vector));

    Ok(())
}

/// Make sure the output directory exists and may be written into.
///
/// It returns `false` when the user refuses to overwrite the files which are already there.
fn prepare_output_directory(args: &CLIArgs, output_files: &OutputFiles) -> anyhow::Result<bool> {
    let metadata = match args.output_path.metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(args.output_path.as_path())
                .with_context(|| anyhow!("{:?}", args.output_path))?;

            return Ok(true);
        },
        Err(error) => {
            return Err(error).with_context(|| anyhow!("{:?}", args.output_path));
        },
    };

    if !metadata.is_dir() {
        return Err(anyhow!("{:?} is not a directory.", args.output_path));
    }

    let mut need_overwrite = false;

    for path in output_files.iter() {
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

    if need_overwrite && !args.overwrite {
        return ask_overwrite();
    }

    Ok(true)
}

/// Ask whether the existing files may be overwritten, and keep asking until the answer can be understood.
fn ask_overwrite() -> anyhow::Result<bool> {
    let mut answer = String::new();

    loop {
        print!("Overwrite files? [Y/N] ");
        io::stdout().flush().with_context(|| "stdout")?;

        answer.clear();

        if io::stdin().read_line(&mut answer).with_context(|| "stdin")? == 0 {
            // the input has ended, so there is no answer to wait for
            return Ok(false);
        }

        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => continue,
        }
    }
}
