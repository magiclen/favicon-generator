mod cli;
mod favicon;
mod html;
mod manifest;

use std::{
    borrow::Cow,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, anyhow, bail};
use cli::*;
use favicon::{LARGEST_ICON_SIZE, OutputFiles};
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

    let svg = favicon::is_svg(&identify);

    let output_files = OutputFiles::new(args.output_path.as_path(), svg);

    if !check_output_directory(&args, &output_files)? {
        bail!("Generation cancelled; no files were written.");
    }

    let sharpen = !svg && !args.no_sharpen;

    let source = favicon::prepare_source(input, &identify, args.fit)
        .with_context(|| anyhow!("{:?}", args.input_path))?;

    if let Some((width, height)) = source
        .raster_size()
        .filter(|&(width, height)| width.max(height) < LARGEST_ICON_SIZE as usize)
    {
        eprintln!(
            "Warning: The icons are enlarged from a {width}x{height} image, so they can look \
             blurry. Use an image of at least {LARGEST_ICON_SIZE}x{LARGEST_ICON_SIZE} pixels or \
             an SVG image."
        );
    }

    let mut generated_files: Vec<(&Path, Cow<'_, [u8]>)> =
        Vec::with_capacity(output_files.iter().count());

    if let Some(svg) = output_files.svg.as_deref() {
        generated_files.push((svg, source.resource.as_u8_slice().unwrap().into()));
    }

    let ico = output_files.ico.as_path();

    generated_files.push((
        ico,
        favicon::generate_ico(&source, args.fit, sharpen)
            .with_context(|| anyhow!("generating {ico:?}"))?
            .into(),
    ));

    let apple_touch_icon = output_files.apple_touch_icon.as_path();

    generated_files.push((
        apple_touch_icon,
        favicon::generate_apple_touch_icon(
            &source,
            args.background_color.as_str(),
            args.fit,
            sharpen,
        )
        .with_context(|| anyhow!("generating {apple_touch_icon:?}"))?
        .into(),
    ));

    for (size, png) in output_files.manifest_icons.iter() {
        generated_files.push((
            png.as_path(),
            favicon::generate_png(&source, *size, args.fit, sharpen)
                .with_context(|| anyhow!("generating {png:?}"))?
                .into(),
        ));
    }

    let maskable_icon = output_files.maskable_icon.as_path();

    generated_files.push((
        maskable_icon,
        favicon::generate_maskable_icon(&source, args.background_color.as_str(), args.fit, sharpen)
            .with_context(|| anyhow!("generating {maskable_icon:?}"))?
            .into(),
    ));

    let mut web_app_manifest = WebAppManifest::new(&args).to_json();

    web_app_manifest.push('\n');

    generated_files
        .push((output_files.web_app_manifest.as_path(), web_app_manifest.into_bytes().into()));

    // Finish every conversion before creating the directory or writing any output file.
    fs::create_dir_all(args.output_path.as_path())
        .with_context(|| anyhow!("{:?}", args.output_path))?;

    write_files(&generated_files)?;

    println!("{}", html::build_head(&args, svg));

    Ok(())
}

/// Write every temporary file before replacing any output file.
///
/// A temporary write error leaves existing files unchanged, but a rename error may leave some files replaced.
fn write_files(files: &[(&Path, Cow<'_, [u8]>)]) -> anyhow::Result<()> {
    let mut replacements = Vec::with_capacity(files.len());

    let result = write_temporary_files(files, &mut replacements).and_then(|()| {
        replacements.iter().try_for_each(|(temporary_path, path)| {
            fs::rename(temporary_path, path).with_context(|| anyhow!("{path:?}"))
        })
    });

    if result.is_err() {
        for (temporary_path, _) in replacements.iter() {
            // a temporary file which has been renamed is not there anymore, so the error is ignored
            let _ = fs::remove_file(temporary_path);
        }
    }

    result
}

/// Write the data of each file into a new temporary file in the same directory.
///
/// `replacements` gets every temporary file which has been created, paired with the path it is going to replace.
fn write_temporary_files(
    files: &[(&Path, Cow<'_, [u8]>)],
    replacements: &mut Vec<(PathBuf, PathBuf)>,
) -> anyhow::Result<()> {
    for (output_path, data) in files {
        // Replace the file which a symbolic link points to, instead of the link itself.
        let path = resolve_output_path(output_path).with_context(|| anyhow!("{output_path:?}"))?;

        // `replacements` has one entry for each earlier file, in the same order.
        if let Some(index) = replacements.iter().position(|(_, replaced)| *replaced == path) {
            bail!("{:?} and {output_path:?} point to the same file.", files[index].0);
        }

        let mut temporary_path = path.clone().into_os_string();

        temporary_path.push(format!(".{}.tmp", process::id()));

        let temporary_path = PathBuf::from(temporary_path);

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temporary_path.as_path())
            .with_context(|| anyhow!("{temporary_path:?}"))?;

        replacements.push((temporary_path.clone(), path.clone()));

        file.write_all(data).with_context(|| anyhow!("{temporary_path:?}"))?;

        // A replaced file keeps its permissions.
        if let Ok(metadata) = path.metadata() {
            file.set_permissions(metadata.permissions())
                .with_context(|| anyhow!("{temporary_path:?}"))?;
        }
    }

    Ok(())
}

/// Follow symbolic links, even dangling ones, so the files they point to are replaced instead of the links.
///
/// The returned path is canonical, so two paths of the same file are equal.
fn resolve_output_path(path: &Path) -> io::Result<PathBuf> {
    let mut path = path.to_path_buf();

    loop {
        match fs::canonicalize(&path) {
            Ok(path) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match fs::symlink_metadata(&path) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        let target = fs::read_link(&path)?;

                        if target.is_absolute() {
                            path = target;
                        } else {
                            path.pop();
                            path.push(target);
                        }
                    },
                    Ok(_) => return Err(error),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        let path = std::path::absolute(&path)?;

                        // `absolute` keeps `..`, so the parent is canonicalized to make the path canonical too.
                        let (Some(parent), Some(file_name)) = (path.parent(), path.file_name())
                        else {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "The output path has no parent or file name.",
                            ));
                        };

                        return Ok(fs::canonicalize(parent)?.join(file_name));
                    },
                    Err(error) => return Err(error),
                }
            },
            Err(error) => return Err(error),
        }
    }
}

/// Check the output directory and ask before overwriting existing files.
///
/// It returns `false` when the user refuses to overwrite the files which are already there.
fn check_output_directory(args: &CLIArgs, output_files: &OutputFiles) -> anyhow::Result<bool> {
    let metadata = match args.output_path.metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(error) => {
            return Err(error).with_context(|| anyhow!("{:?}", args.output_path));
        },
    };

    if !metadata.is_dir() {
        return Err(anyhow!("{:?} is not a directory.", args.output_path));
    }

    let mut existing_files = Vec::new();

    for path in output_files.iter() {
        match path.metadata() {
            Ok(metadata) => {
                if metadata.is_dir() {
                    return Err(anyhow!("{path:?} is a directory."));
                }

                existing_files.push(path);
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // do nothing
            },
            Err(error) => {
                return Err(error).with_context(|| anyhow!("{path:?}"));
            },
        }
    }

    if !existing_files.is_empty() && !args.overwrite {
        return ask_overwrite(&existing_files);
    }

    Ok(true)
}

/// List the existing files and ask whether they may be overwritten, and keep asking until the answer can be understood.
fn ask_overwrite(existing_files: &[&Path]) -> anyhow::Result<bool> {
    eprintln!("These files already exist:");

    for path in existing_files {
        eprintln!("  {}", path.display());
    }

    let mut answer = String::new();

    loop {
        eprint!("Overwrite them? [Y/N] ");
        io::stderr().flush().with_context(|| "stderr")?;

        answer.clear();

        if io::stdin().read_line(&mut answer).with_context(|| "stdin")? == 0 {
            // the input has ended, so there is no answer to wait for, and the next message has to start on a new line
            eprintln!();

            return Ok(false);
        }

        match answer.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => continue,
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;

    struct TestDirectory(PathBuf);

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn writing_files_keeps_symbolic_links() {
        let directory =
            std::env::temp_dir().join(format!("favicon-generator-links-{}", process::id()));

        fs::create_dir(&directory).unwrap();

        let directory = TestDirectory(directory);
        let links = directory.0.join("links");
        let targets = directory.0.join("targets");

        fs::create_dir(&links).unwrap();
        fs::create_dir(&targets).unwrap();

        for (name, existing, absolute) in [
            ("existing-relative", true, false),
            ("existing-absolute", true, true),
            ("missing-relative", false, false),
            ("missing-absolute", false, true),
        ] {
            let path = links.join(name);
            let target = targets.join(name);
            let link_target =
                if absolute { target.clone() } else { Path::new("../targets").join(name) };

            if existing {
                fs::write(&target, b"old data").unwrap();
            }

            symlink(&link_target, &path).unwrap();
            write_files(&[(path.as_path(), Cow::Borrowed(b"new data"))]).unwrap();

            assert!(fs::symlink_metadata(&path).unwrap().file_type().is_symlink());
            assert_eq!(link_target, fs::read_link(&path).unwrap());
            assert_eq!(b"new data".as_slice(), fs::read(&target).unwrap());
        }
    }
}
