use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use clap::ValueEnum;
use image_convert::{
    Crop, ICOConfig, ImageIdentify, ImageResource, InterlaceType, PNGConfig,
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

/// The size of the largest icon. A raster image smaller than this is enlarged, so the icons can look blurry.
pub const LARGEST_ICON_SIZE: u32 = 512;

/// The sizes of the icons which the web app manifest points at. Chrome asks for both of them before it offers to install a web app.
pub const MANIFEST_ICON_SIZES: &[u32] = &[192, LARGEST_ICON_SIZE];

/// The size of the maskable icon, which Android crops into the shape of its launcher.
pub const MASKABLE_ICON_SIZE: u32 = 512;

/// The diameter of the safe circle of the maskable icon, as a ratio of the icon width. The launcher may crop everything outside this circle.
const MASKABLE_SAFE_ZONE_RATIO: f64 = 0.8;

/// Pixels which are less opaque than this do not count as a part of the maskable logo, so faint noise and resampling halos do not make the logo smaller.
const MASKABLE_VISIBLE_ALPHA: u8 = 8;

/// How much of the width of the Apple touch icon the logo is allowed to take. iOS draws the icon without any padding of its own, so the padding has to be a part of the image.
const APPLE_TOUCH_ICON_CONTENT_RATIO: f64 = 0.8;

/// How an image fits into a square icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FitMode {
    Contain,
    Cover,
}

/// The source data or decoded pixels shared by the output icons.
#[derive(Debug)]
pub struct SourceImage {
    pub resource: ImageResource,
    square:       bool,
}

impl SourceImage {
    /// The size of the decoded image. It is `None` for an SVG image, which is rendered again at every icon size.
    pub fn raster_size(&self) -> Option<(usize, usize)> {
        self.resource.as_magick_wand().map(|mw| (mw.get_image_width(), mw.get_image_height()))
    }
}

/// The file name of the manifest icon of the given size.
#[inline]
pub fn manifest_icon_file_name(size: u32) -> String {
    format!("icon-{size}.png")
}

/// The paths of the files which are going to be written.
#[derive(Debug)]
pub struct OutputFiles {
    pub ico:              PathBuf,
    /// It is `Some` only when the input image is an SVG image.
    pub svg:              Option<PathBuf>,
    pub apple_touch_icon: PathBuf,
    pub manifest_icons:   Vec<(u32, PathBuf)>,
    pub maskable_icon:    PathBuf,
    pub web_app_manifest: PathBuf,
}

impl OutputFiles {
    pub fn new(output_path: &Path, svg: bool) -> OutputFiles {
        OutputFiles {
            ico:              output_path.join(FILE_FAVICON_ICO),
            svg:              svg.then(|| output_path.join(FILE_FAVICON_SVG)),
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

/// Whether the input image is an SVG image. It is copied as the SVG icon, and **ImageMagick** renders it again at the size of every other icon.
#[inline]
pub fn is_svg(identify: &ImageIdentify) -> bool {
    identify.format == "SVG"
}

/// Keep SVG data for each output size and decode other images only once.
pub fn prepare_source(
    input: ImageResource,
    identify: &ImageIdentify,
    fit: FitMode,
) -> anyhow::Result<SourceImage> {
    let square = identify.resolution.width == identify.resolution.height;

    if is_svg(identify) {
        return Ok(SourceImage {
            resource: input,
            square,
        });
    }

    let mut config = PNGConfig::new();

    if fit == FitMode::Cover {
        config.crop = Some(Crop::Center(1f64, 1f64));
    }

    let (mut mw, _) =
        image_convert::fetch_magic_wand(&input, &config).with_context(|| "reading the image")?;

    // Icons do not need 16-bit channels, and the PNG encoder writes the depth of the image.
    mw.set_image_depth(8)?;

    Ok(SourceImage {
        square:   mw.get_image_width() == mw.get_image_height(),
        resource: ImageResource::MagickWand(mw),
    })
}

/// Generate the data of `favicon.ico`.
pub fn generate_ico(source: &SourceImage, fit: FitMode, sharpen: bool) -> anyhow::Result<Vec<u8>> {
    let logo = render_logo(source, ICO_SIZE, fit, sharpen)?;
    let mut canvas = fit_canvas(ICO_SIZE, logo).with_context(|| "drawing the ICO icon")?;

    // RGBA data needs pixel order instead of the line order used for PNG rendering.
    canvas.set_interlace_scheme(InterlaceType::No)?;

    let mut config = ICOConfig::new();

    // The logo has been resized and sharpened already.
    config.sharpen = 0f64;
    config.size.push((ICO_SIZE, ICO_SIZE));

    let mut output = ImageResource::Data(Vec::new());

    to_ico(&mut output, &ImageResource::MagickWand(canvas), &config)
        .with_context(|| "generating the ICO icon")?;

    Ok(output.into_vec().unwrap())
}

/// Generate a PNG icon which keeps the transparency of the input image.
pub fn generate_png(
    source: &SourceImage,
    size: u32,
    fit: FitMode,
    sharpen: bool,
) -> anyhow::Result<Vec<u8>> {
    let logo = render_logo(source, size, fit, sharpen)?;
    let canvas = fit_canvas(size, logo).with_context(|| "drawing the PNG icon")?;

    encode_png(canvas)
}

/// Generate the Apple touch icon with padding and an opaque background.
pub fn generate_apple_touch_icon(
    source: &SourceImage,
    background_color: &str,
    fit: FitMode,
    sharpen: bool,
) -> anyhow::Result<Vec<u8>> {
    generate_padded_png(
        source,
        APPLE_TOUCH_ICON_SIZE,
        (f64::from(APPLE_TOUCH_ICON_SIZE) * APPLE_TOUCH_ICON_CONTENT_RATIO).round() as u32,
        background_color,
        fit,
        sharpen,
    )
}

/// Generate the maskable icon with its logo inside the safe circle and an opaque background.
pub fn generate_maskable_icon(
    source: &SourceImage,
    background_color: &str,
    fit: FitMode,
    sharpen: bool,
) -> anyhow::Result<Vec<u8>> {
    let content_size = maskable_content_size(source, fit)?;

    generate_padded_png(source, MASKABLE_ICON_SIZE, content_size, background_color, fit, sharpen)
}

/// The size of the box which the maskable logo is drawn in. It is as large as the visible pixels of the logo allow while they stay inside the safe circle.
fn maskable_content_size(source: &SourceImage, fit: FitMode) -> anyhow::Result<u32> {
    let safe_diameter = f64::from(MASKABLE_ICON_SIZE) * MASKABLE_SAFE_ZONE_RATIO;

    // Even sizes keep the logo centered.
    let largest_size = safe_diameter.floor() as u32 & !1;

    // Measure the logo at the largest size, and add one pixel for the small differences between this rendering and the final one.
    let logo = render_logo(source, largest_size, fit, false)?;
    let radius = visible_radius(&logo)? + 1f64;

    let size = (f64::from(largest_size) * safe_diameter / 2f64 / radius).floor() as u32;

    Ok(size.min(largest_size) & !1)
}

/// The distance from the center of the logo to the farthest corner of its visible pixels.
fn visible_radius(logo: &MagickWand) -> anyhow::Result<f64> {
    let width = logo.get_image_width();
    let height = logo.get_image_height();

    let alpha = logo
        .export_image_pixels(0, 0, width, height, "A")
        .ok_or_else(|| anyhow!("Cannot read the alpha channel of the logo."))?;

    let center_x = width as f64 / 2f64;
    let center_y = height as f64 / 2f64;

    let mut radius = 0f64;

    for (index, value) in alpha.into_iter().enumerate() {
        if value < MASKABLE_VISIBLE_ALPHA {
            continue;
        }

        // The pixel covers the square from (x, y) to (x + 1, y + 1).
        let x = (index % width) as f64;
        let y = (index / width) as f64;
        let dx = (x - center_x).abs().max((x + 1f64 - center_x).abs());
        let dy = (y - center_y).abs().max((y + 1f64 - center_y).abs());

        radius = radius.max(dx.hypot(dy));
    }

    Ok(radius)
}

/// Generate a PNG icon with a smaller logo centered on an opaque background.
fn generate_padded_png(
    source: &SourceImage,
    size: u32,
    content_size: u32,
    background_color: &str,
    fit: FitMode,
    sharpen: bool,
) -> anyhow::Result<Vec<u8>> {
    let mut logo = render_logo(source, content_size, fit, sharpen)?;

    // Resizing can push HDRI pixels out of range, so clamp them before the logo is drawn onto an opaque background, like PNG encoding would do.
    logo.set_image_depth(8)?;

    let canvas = new_canvas(size, Some(background_color), &logo)
        .with_context(|| "drawing the padded icon")?;

    encode_png(canvas)
}

/// Resize or render the logo without encoding an intermediate PNG image.
fn render_logo(
    source: &SourceImage,
    size: u32,
    fit: FitMode,
    sharpen: bool,
) -> anyhow::Result<MagickWand> {
    to_png_wand(&source.resource, &png_config(source, size, fit, sharpen))
        .with_context(|| "rendering the logo")
}

/// Encode the finished canvas without resizing or sharpening it again.
fn encode_png(canvas: MagickWand) -> anyhow::Result<Vec<u8>> {
    let mut config = PNGConfig::new();

    config.sharpen = 0f64;

    let mut png = to_png_wand(&ImageResource::MagickWand(canvas), &config)
        .with_context(|| "encoding the PNG icon")?;

    // `to_png` sets Adam7 interlacing, which only makes a small icon bigger.
    png.set_interlace_scheme(InterlaceType::No)?;

    png.write_image_blob("PNG").with_context(|| "encoding the PNG icon")
}

/// Run `to_png` into a `MagickWand` instead of PNG data, so the result can still be changed before it is encoded.
fn to_png_wand(
    input: &ImageResource,
    config: &PNGConfig,
) -> Result<MagickWand, image_convert::MagickError> {
    image_convert::start_call_once();

    let mut output = ImageResource::MagickWand(MagickWand::new());

    to_png(&mut output, input, config)?;

    Ok(output.into_magick_wand().unwrap())
}

/// Use the logo as the icon when it fills the whole square, or center it on a transparent canvas.
fn fit_canvas(size: u32, logo: MagickWand) -> Result<MagickWand, image_convert::MagickError> {
    if logo.get_image_width() == size as usize && logo.get_image_height() == size as usize {
        Ok(logo)
    } else {
        new_canvas(size, None, &logo)
    }
}

/// Center the logo on a square canvas, using transparency when no background color is given.
fn new_canvas(
    size: u32,
    background_color: Option<&str>,
    logo: &MagickWand,
) -> Result<MagickWand, image_convert::MagickError> {
    let mut background = PixelWand::new();

    background.set_color(background_color.unwrap_or("none"))?;

    let mut canvas = MagickWand::new();

    canvas.new_image(size as usize, size as usize, &background)?;
    canvas.set_image_depth(logo.get_image_depth())?;

    let x = ((size as usize).saturating_sub(logo.get_image_width()) / 2) as isize;
    let y = ((size as usize).saturating_sub(logo.get_image_height()) / 2) as isize;

    canvas.compose_images(logo, CompositeOperator::Over, false, x, y)?;

    if background_color.is_some() {
        canvas.set_image_alpha_channel(AlphaChannelOption::Remove)?;
    }

    Ok(canvas)
}

/// The size, fit mode and sharpening settings used to render an icon.
fn png_config(source: &SourceImage, size: u32, fit: FitMode, sharpen: bool) -> PNGConfig {
    let mut config = PNGConfig::new();

    config.shrink_only = false;
    config.width = size;
    config.height = size;

    if fit == FitMode::Cover && !source.square {
        config.crop = Some(Crop::Center(1f64, 1f64));
    }

    if !sharpen {
        config.sharpen = 0f64;
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDE_LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><path fill="#f00" d="M0 0h10v20H0z"/><path fill="#0f0" d="M10 0h20v20H10z"/><path fill="#00f" d="M30 0h10v20H30z"/></svg>"##;
    const TALL_LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="40" viewBox="0 0 20 40"><path fill="#f00" d="M0 0h20v10H0z"/><path fill="#0f0" d="M0 10h20v20H0z"/><path fill="#00f" d="M0 30h20v10H0z"/></svg>"##;
    const SQUARE_LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40"><path fill="#f00" d="M0 0h40v40H0z"/></svg>"##;
    const ROUND_LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40"><circle cx="20" cy="20" r="20" fill="#f00"/></svg>"##;

    fn source_images(svg: &str, fit: FitMode) -> [SourceImage; 2] {
        let svg = ImageResource::Data(svg.as_bytes().to_vec());
        let mut raster = ImageResource::Data(Vec::new());

        to_png(&mut raster, &svg, &PNGConfig::new()).unwrap();

        [svg, raster].map(|input| {
            let identify = image_convert::identify_ping(&input).unwrap();

            prepare_source(input, &identify, fit).unwrap()
        })
    }

    fn read_icon(data: Vec<u8>) -> MagickWand {
        image_convert::fetch_magic_wand(&ImageResource::Data(data), &PNGConfig::new()).unwrap().0
    }

    fn pixel(image: &MagickWand, x: isize, y: isize) -> [u8; 4] {
        image.export_image_pixels(x, y, 1, 1, "RGBA").unwrap().try_into().unwrap()
    }

    #[test]
    fn contain_keeps_and_centers_a_wide_logo() {
        for source in source_images(WIDE_LOGO, FitMode::Contain) {
            let image = read_icon(generate_png(&source, 64, FitMode::Contain, false).unwrap());

            assert_eq!((64, 64), (image.get_image_width(), image.get_image_height()));
            assert_eq!(0, pixel(&image, 32, 0)[3]);
            assert_eq!(0, pixel(&image, 32, 63)[3]);
            assert_eq!([255, 0, 0, 255], pixel(&image, 4, 32));
            assert_eq!([0, 255, 0, 255], pixel(&image, 32, 32));
            assert_eq!([0, 0, 255, 255], pixel(&image, 60, 32));
        }
    }

    #[test]
    fn contain_keeps_and_centers_a_tall_logo() {
        for source in source_images(TALL_LOGO, FitMode::Contain) {
            let image = read_icon(generate_png(&source, 64, FitMode::Contain, false).unwrap());

            assert_eq!((64, 64), (image.get_image_width(), image.get_image_height()));
            assert_eq!(0, pixel(&image, 0, 32)[3]);
            assert_eq!(0, pixel(&image, 63, 32)[3]);
            assert_eq!([255, 0, 0, 255], pixel(&image, 32, 4));
            assert_eq!([0, 255, 0, 255], pixel(&image, 32, 32));
            assert_eq!([0, 0, 255, 255], pixel(&image, 32, 60));
        }
    }

    #[test]
    fn cover_crops_the_center_of_a_logo() {
        for svg in [WIDE_LOGO, TALL_LOGO] {
            for source in source_images(svg, FitMode::Cover) {
                let image = read_icon(generate_png(&source, 64, FitMode::Cover, false).unwrap());

                assert_eq!((64, 64), (image.get_image_width(), image.get_image_height()));
                assert_eq!([0, 255, 0, 255], pixel(&image, 4, 32));
                assert_eq!([0, 255, 0, 255], pixel(&image, 32, 4));
                assert_eq!([0, 255, 0, 255], pixel(&image, 60, 32));
                assert_eq!([0, 255, 0, 255], pixel(&image, 32, 60));
            }
        }
    }

    #[test]
    fn ico_keeps_the_size_colors_and_padding() {
        for svg in [WIDE_LOGO, TALL_LOGO, SQUARE_LOGO] {
            for fit in [FitMode::Contain, FitMode::Cover] {
                for source in source_images(svg, fit) {
                    let image = read_icon(generate_ico(&source, fit, false).unwrap());

                    assert_eq!((32, 32), (image.get_image_width(), image.get_image_height()));
                    assert_eq!(1, image.get_number_images());

                    let color =
                        if svg == SQUARE_LOGO { [255, 0, 0, 255] } else { [0, 255, 0, 255] };

                    assert_eq!(color, pixel(&image, 16, 16));

                    if fit == FitMode::Contain && svg != SQUARE_LOGO {
                        let edge = if svg == WIDE_LOGO {
                            pixel(&image, 16, 0)
                        } else {
                            pixel(&image, 0, 16)
                        };

                        assert_eq!(0, edge[3]);
                    } else {
                        assert_eq!(255, pixel(&image, 16, 0)[3]);
                        assert_eq!(255, pixel(&image, 0, 16)[3]);
                    }
                }
            }
        }
    }

    #[test]
    fn padded_icons_are_opaque_and_keep_the_maskable_logo_inside_the_safe_circle() {
        for source in source_images(SQUARE_LOGO, FitMode::Contain) {
            let apple = read_icon(
                generate_apple_touch_icon(&source, "#ffffff", FitMode::Contain, false).unwrap(),
            );

            assert_eq!((180, 180), (apple.get_image_width(), apple.get_image_height()));
            assert!(!apple.get_image_alpha_channel());
            assert_eq!([255, 255, 255, 255], pixel(&apple, 17, 90));
            assert_eq!([255, 0, 0, 255], pixel(&apple, 18, 90));
            assert_eq!([255, 0, 0, 255], pixel(&apple, 161, 90));
            assert_eq!([255, 255, 255, 255], pixel(&apple, 162, 90));

            let mask = read_icon(
                generate_maskable_icon(&source, "#ffffff", FitMode::Contain, false).unwrap(),
            );

            assert_eq!((512, 512), (mask.get_image_width(), mask.get_image_height()));
            assert!(!mask.get_image_alpha_channel());

            let pixels = mask.export_image_pixels(0, 0, 512, 512, "RGBA").unwrap();
            let mut logo_pixels = 0;

            for (index, pixel) in pixels.chunks_exact(4).enumerate() {
                assert_eq!(255, pixel[3]);

                if pixel == [255, 0, 0, 255] {
                    let x = (index % 512) as f64 + 0.5 - 256f64;
                    let y = (index / 512) as f64 + 0.5 - 256f64;

                    assert!(x * x + y * y <= 204.8f64 * 204.8f64);
                    logo_pixels += 1;
                }
            }

            assert_eq!(288 * 288, logo_pixels);
        }
    }

    #[test]
    fn maskable_icon_enlarges_a_round_logo_inside_the_safe_circle() {
        for source in source_images(ROUND_LOGO, FitMode::Contain) {
            let mask = read_icon(
                generate_maskable_icon(&source, "#ffffff", FitMode::Contain, false).unwrap(),
            );

            // a square logo is drawn 288 pixels wide, so its edge is only 144 pixels away from the center
            assert_eq!([255, 0, 0, 255], pixel(&mask, 256 - 150, 256));
            assert_eq!([255, 0, 0, 255], pixel(&mask, 256, 256 + 150));

            let pixels = mask.export_image_pixels(0, 0, 512, 512, "RGBA").unwrap();

            for (index, pixel) in pixels.chunks_exact(4).enumerate() {
                let x = (index % 512) as f64 + 0.5 - 256f64;
                let y = (index / 512) as f64 + 0.5 - 256f64;

                if x * x + y * y > 204.8f64 * 204.8f64 {
                    // only invisible traces of the red logo can be outside the safe circle
                    assert!(pixel[1] > 255 - MASKABLE_VISIBLE_ALPHA);
                }
            }
        }
    }

    #[test]
    fn icons_of_a_16_bit_image_are_at_most_8_bit() {
        image_convert::start_call_once();

        let mut color = PixelWand::new();

        color.set_color("#123456789ABC").unwrap();

        let mut image = MagickWand::new();

        image.new_image(40, 40, &color).unwrap();
        image.set_image_depth(16).unwrap();

        let data = image.write_image_blob("PNG").unwrap();

        assert_eq!(16, read_icon(data.clone()).get_image_depth());

        let input = ImageResource::Data(data);
        let identify = image_convert::identify_ping(&input).unwrap();
        let source = prepare_source(input, &identify, FitMode::Contain).unwrap();

        for icon in [
            generate_png(&source, 64, FitMode::Contain, false).unwrap(),
            generate_apple_touch_icon(&source, "#ffffff", FitMode::Contain, false).unwrap(),
            generate_maskable_icon(&source, "#ffffff", FitMode::Contain, false).unwrap(),
        ] {
            // the PNG encoder can store a few colors with fewer bits
            assert!(read_icon(icon).get_image_depth() <= 8);
        }
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
    fn output_files_of_an_svg_input() {
        let identify =
            image_convert::identify_ping(&ImageResource::Data(WIDE_LOGO.as_bytes().to_vec()))
                .unwrap();
        let files = OutputFiles::new(Path::new("/out"), is_svg(&identify));

        assert_eq!(Some(Path::new("/out/favicon.svg")), files.svg.as_deref());
        assert_eq!(7, files.iter().count());
    }
}
