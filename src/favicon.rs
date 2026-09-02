use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};
use image_convert::{
    Crop, ICOConfig, ImageIdentify, ImageResource, PNGConfig,
    magick_rust::{AlphaChannelOption, CompositeOperator, MagickWand, PixelWand},
    to_ico, to_png,
};

pub const FILE_FAVICON_ICO: &str = "favicon.ico";
pub const FILE_FAVICON_SVG: &str = "favicon.svg";
pub const FILE_APPLE_TOUCH_ICON: &str = "apple-touch-icon.png";
pub const FILE_MASKABLE_ICON: &str = "icon-mask.png";
pub const FILE_WEB_APP_MANIFEST: &str = "manifest.webmanifest";

/// The size of the only image in `favicon.ico`. A browser which supports the SVG icon never reads this file, and one which does not never asks for more than 32 pixels.
pub const ICO_SIZE: u32 = 32;

/// The size of the icon which iOS uses for a home screen shortcut. iOS 8 and later scale this one down instead of asking for the smaller sizes.
pub const APPLE_TOUCH_ICON_SIZE: u32 = 180;

/// The sizes of the icons which the web app manifest points at. Chrome asks for both of them before it offers to install a web app.
pub const MANIFEST_ICON_SIZES: &[u32] = &[192, 512];

/// The size of the maskable icon, which Android crops into the shape of its launcher.
pub const MASKABLE_ICON_SIZE: u32 = 512;

/// How much of the width of the maskable icon the logo is allowed to take.
///
/// The safe zone of the specification is a centered circle whose radius is 40% of the width, and 80% of the width is the ratio which the common generators fit their logo into.
const MASKABLE_CONTENT_RATIO: f64 = 0.8;

/// How much of the width of the Apple touch icon the logo is allowed to take. iOS draws the icon without any padding of its own, so the padding has to be a part of the image.
const APPLE_TOUCH_ICON_CONTENT_RATIO: f64 = 0.8;

/// The biggest size which any output file needs. A vector image is rendered at least this large, so that no output has to be enlarged.
const LARGEST_OUTPUT_SIZE: u32 = 512;

/// The file name of the manifest icon of the given size.
#[inline]
pub fn manifest_icon_file_name(size: u32) -> String {
    format!("icon-{size}.png")
}

/// The paths of the files which are going to be written.
#[derive(Debug)]
pub struct OutputFiles {
    pub ico:              PathBuf,
    /// It is `Some` only when the input image is a vector image, which is the only case where an SVG icon can be written.
    pub svg:              Option<PathBuf>,
    pub apple_touch_icon: PathBuf,
    pub manifest_icons:   Vec<(u32, PathBuf)>,
    pub maskable_icon:    PathBuf,
    pub web_app_manifest: PathBuf,
}

impl OutputFiles {
    pub fn new(output_path: &Path, vector: bool) -> OutputFiles {
        OutputFiles {
            ico:              output_path.join(FILE_FAVICON_ICO),
            svg:              vector.then(|| output_path.join(FILE_FAVICON_SVG)),
            apple_touch_icon: output_path.join(FILE_APPLE_TOUCH_ICON),
            manifest_icons:   MANIFEST_ICON_SIZES
                .iter()
                .copied()
                .map(|size| (size, output_path.join(manifest_icon_file_name(size))))
                .collect(),
            maskable_icon:    output_path.join(FILE_MASKABLE_ICON),
            web_app_manifest: output_path.join(FILE_WEB_APP_MANIFEST),
        }
    }

    /// Every path this struct holds. The overwrite check walks through it, so a new output file is never missed.
    pub fn iter(&self) -> impl Iterator<Item = &Path> {
        [
            Some(&self.ico),
            self.svg.as_ref(),
            Some(&self.apple_touch_icon),
            Some(&self.maskable_icon),
            Some(&self.web_app_manifest),
        ]
        .into_iter()
        .flatten()
        .chain(self.manifest_icons.iter().map(|(_, path)| path))
        .map(PathBuf::as_path)
    }
}

/// Whether **ImageMagick** reads the image as a vector image, which it can render again in any size.
#[inline]
pub fn is_vector(identify: &ImageIdentify) -> bool {
    matches!(identify.format.as_str(), "SVG" | "MVG")
}

/// Turn the input image into the square source image which every output file is generated from.
///
/// A square vector image is returned untouched, so that **ImageMagick** renders it again for every output size instead of resizing one rendering of it.
pub fn prepare_source(
    mut input: ImageResource,
    identify: &ImageIdentify,
) -> anyhow::Result<ImageResource> {
    let square = identify.resolution.width == identify.resolution.height;

    if is_vector(identify) {
        if square {
            return Ok(input);
        }

        // a vector image has to be rendered before it is cropped, otherwise the crop result would be thrown away, so render it large enough for the biggest output size
        let (width, height) =
            cover_size(identify.resolution.width, identify.resolution.height, LARGEST_OUTPUT_SIZE);

        let mut config = PNGConfig::new();

        config.shrink_only = false;
        config.width = width;
        config.height = height;

        let (mw, _) = image_convert::fetch_magic_wand(&input, &config)
            .with_context(|| "rendering the vector image")?;

        input = ImageResource::MagickWand(mw);
    }

    // only the `ImageConfig` part of the config is read here, so any output format would do
    let mut config = PNGConfig::new();

    if !square {
        config.crop = Some(Crop::Center(1f64, 1f64));
    }

    let (mw, _) =
        image_convert::fetch_magic_wand(&input, &config).with_context(|| "reading the image")?;

    Ok(ImageResource::MagickWand(mw))
}

/// Compute a size whose shorter side is `size` and whose aspect ratio is the given one, so that cropping it into a square leaves a `size` by `size` image.
fn cover_size(width: u32, height: u32, size: u32) -> (u32, u32) {
    let shorter = width.min(height);

    if shorter == 0 {
        return (size, size);
    }

    let scale = f64::from(size) / f64::from(shorter);

    ((f64::from(width) * scale).round() as u32, (f64::from(height) * scale).round() as u32)
}

/// Write the input image as `favicon.svg` without touching it.
///
/// The data is written instead of the file being copied, so that an input image which lives in the output directory under this name is not truncated before it is read.
pub fn generate_svg(output: &Path, data: &[u8]) -> anyhow::Result<()> {
    fs::write(output, data).with_context(|| anyhow!("{output:?}"))
}

/// Write `favicon.ico`.
pub fn generate_ico(output: &Path, source: &ImageResource, sharpen: bool) -> anyhow::Result<()> {
    let mut config = ICOConfig::new();

    if !sharpen {
        config.sharpen = 0f64;
    }

    config.size.push((ICO_SIZE, ICO_SIZE));

    let mut output_resource = ImageResource::from_path(output);

    to_ico(&mut output_resource, source, &config).with_context(|| anyhow!("to_ico {output:?}"))
}

/// Write a PNG icon which keeps the transparency of the input image.
pub fn generate_png(
    output: &Path,
    source: &ImageResource,
    size: u32,
    sharpen: bool,
) -> anyhow::Result<()> {
    let config = png_config(size, sharpen);

    let mut output_resource = ImageResource::from_path(output);

    to_png(&mut output_resource, source, &config).with_context(|| anyhow!("to_png {output:?}"))
}

/// Write the Apple touch icon, whose logo is padded and drawn on an opaque background.
pub fn generate_apple_touch_icon(
    output: &Path,
    source: &ImageResource,
    background_color: &str,
    sharpen: bool,
) -> anyhow::Result<()> {
    generate_padded_png(
        output,
        source,
        APPLE_TOUCH_ICON_SIZE,
        APPLE_TOUCH_ICON_CONTENT_RATIO,
        background_color,
        sharpen,
    )
}

/// Write the maskable icon, whose logo is kept inside the safe zone and drawn on an opaque background.
pub fn generate_maskable_icon(
    output: &Path,
    source: &ImageResource,
    background_color: &str,
    sharpen: bool,
) -> anyhow::Result<()> {
    generate_padded_png(
        output,
        source,
        MASKABLE_ICON_SIZE,
        MASKABLE_CONTENT_RATIO,
        background_color,
        sharpen,
    )
}

/// Write a PNG icon whose logo takes `content_ratio` of the width and is centered on an opaque background.
///
/// iOS fills the transparency of an Apple touch icon with black, and Android crops a maskable icon into the shape of its launcher, so neither of them may be transparent.
fn generate_padded_png(
    output: &Path,
    source: &ImageResource,
    size: u32,
    content_ratio: f64,
    background_color: &str,
    sharpen: bool,
) -> anyhow::Result<()> {
    let content_size = (f64::from(size) * content_ratio).round() as u32;

    let mut logo = ImageResource::Data(Vec::new());

    to_png(&mut logo, source, &png_config(content_size, sharpen))
        .with_context(|| anyhow!("to_png {output:?}"))?;

    let logo_wand = MagickWand::new();

    logo_wand
        .read_image_blob(logo.into_vec().unwrap())
        .with_context(|| anyhow!("reading the logo of {output:?}"))?;

    let canvas = new_canvas(size, background_color, &logo_wand)
        .with_context(|| anyhow!("drawing {output:?}"))?;

    // the logo has been sharpened already, and the canvas is written out in the size it was drawn in
    let mut config = PNGConfig::new();

    config.sharpen = 0f64;

    let mut output_resource = ImageResource::from_path(output);

    to_png(&mut output_resource, &ImageResource::MagickWand(canvas), &config)
        .with_context(|| anyhow!("to_png {output:?}"))
}

/// Draw the logo in the middle of an opaque square canvas of the given color.
fn new_canvas(
    size: u32,
    background_color: &str,
    logo: &MagickWand,
) -> Result<MagickWand, image_convert::MagickError> {
    let mut background = PixelWand::new();

    background.set_color(background_color)?;

    let canvas = MagickWand::new();

    canvas.new_image(size as usize, size as usize, &background)?;

    let offset = ((size as usize).saturating_sub(logo.get_image_width()) / 2) as isize;

    canvas.compose_images(logo, CompositeOperator::Over, false, offset, offset)?;

    // the canvas is opaque already, so dropping the alpha channel only makes the output smaller
    canvas.set_image_alpha_channel(AlphaChannelOption::Remove)?;

    Ok(canvas)
}

/// The config of a PNG icon of the given size. The source image is already square, so no cropping is left to do.
fn png_config(size: u32, sharpen: bool) -> PNGConfig {
    let mut config = PNGConfig::new();

    config.shrink_only = false;
    config.width = size;
    config.height = size;

    if !sharpen {
        config.sharpen = 0f64;
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_size_of_a_square_image() {
        assert_eq!((512, 512), cover_size(100, 100, 512));
    }

    #[test]
    fn cover_size_of_a_wide_image() {
        assert_eq!((1536, 512), cover_size(300, 100, 512));
    }

    #[test]
    fn cover_size_of_a_tall_image() {
        assert_eq!((512, 1024), cover_size(100, 200, 512));
    }

    #[test]
    fn output_files_of_a_raster_input() {
        let files = OutputFiles::new(Path::new("/out"), false);

        assert_eq!(
            vec![
                Path::new("/out/favicon.ico"),
                Path::new("/out/apple-touch-icon.png"),
                Path::new("/out/icon-mask.png"),
                Path::new("/out/manifest.webmanifest"),
                Path::new("/out/icon-192.png"),
                Path::new("/out/icon-512.png"),
            ],
            files.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn output_files_of_a_vector_input() {
        let files = OutputFiles::new(Path::new("/out"), true);

        assert_eq!(Some(Path::new("/out/favicon.svg")), files.svg.as_deref());
        assert_eq!(7, files.iter().count());
    }
}
