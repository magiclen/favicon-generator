Favicon Generator
====================

[![CI](https://github.com/magiclen/favicon-generator/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/favicon-generator/actions/workflows/ci.yml)

It helps you generate favicons with different formats and sizes.

## Installation

Building requires Rust 1.85 or later, ImageMagick 7.1.1-31 or newer in the 7.1 series with its development headers, and Clang/libclang. `pkg-config` finds the ImageMagick headers and libraries unless you set the `IMAGE_MAGICK_*` variables below. ImageMagick needs PNG support; its other delegates decide which input formats are available. The librsvg delegate is recommended for SVG rendering.

### Linux

On Debian or Ubuntu, install the build dependencies and build ImageMagick 7 if your distribution only provides ImageMagick 6:

```sh
sudo apt update
sudo apt install build-essential clang libclang-dev pkg-config librsvg2-dev libpng-dev libjpeg-dev libwebp-dev
wget https://download.imagemagick.org/archive/ImageMagick.tar.gz
tar xf ImageMagick.tar.gz
cd ImageMagick-*
./configure --enable-hdri --with-rsvg
make -j"$(nproc)"
sudo make install
sudo ldconfig
```

See the [ImageMagick source installation guide](https://imagemagick.org/install-source/) and the [bindgen requirements](https://rust-lang.github.io/rust-bindgen/requirements.html) for other distributions.

### macOS

Use the [Homebrew `imagemagick-full` formula](https://formulae.brew.sh/formula/imagemagick-full), which includes librsvg:

```sh
brew install imagemagick-full pkgconf llvm
export PATH="$(brew --prefix imagemagick-full)/bin:$PATH"
export PKG_CONFIG_PATH="$(brew --prefix imagemagick-full)/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
```

### Install the program

After installing the native dependencies, install a published release from crates.io:

```sh
cargo install favicon-generator --locked
```

To install the source in this checkout, run from the project directory:

```sh
cargo install --path . --locked
```

### Static musl build

Use ImageMagick and all of its delegate libraries built for musl. For the library set used by this project's Makefile, replace the ImageMagick prefix and adjust the musl and GCC directories to match your installation:

```sh
rustup target add x86_64-unknown-linux-musl
export IMAGE_MAGICK_INCLUDE_DIRS="/path/to/ImageMagick-linux/linux/include/ImageMagick-7"
export IMAGE_MAGICK_LIB_DIRS="/path/to/ImageMagick-linux/linux/lib:/opt/musl/x86_64-linux-musl/lib:/opt/musl/lib/gcc/x86_64-linux-musl/15.1.0"
export IMAGE_MAGICK_LIBS="MagickWand-7.Q16HDRI:jbig:lcms2:m:pthread:tiff:webp:sharpyuv:zstd:lzma:jpeg:z:freetype:bz2:png16:harfbuzz:brotlidec:brotlicommon:raqm:fribidi:jxl:stdc++:hwy:brotlienc:jxl_cms:jxl_threads:fontconfig:xml2:iconv:heif:de265:x265:gcc:aom:dav1d:dl:openjp2:webpmux:webpdemux:raw_r:gomp:MagickCore-7.Q16HDRI:stdc++:gcc"
export IMAGE_MAGICK_STATIC=1
cargo build --locked --release --target x86_64-unknown-linux-musl
```

The executable is `target/x86_64-unknown-linux-musl/release/favicon-generator`. The Makefile runs the same build in the maintainer's environment: `MAGICK_PATH` has to be a directory whose `build.sh` builds ImageMagick into `MAGICK_PATH/linux`, and `MUSL_PATH` has to be the musl toolchain directory.

## Generated Files

| File | Size | Purpose |
| ---- | ---- | ------- |
| `favicon.ico` | 32x32 | Browsers which do not support the SVG icon, and the automatic `/favicon.ico` request. |
| `favicon.svg` | - | The icon of any size, generated only when the input image is an SVG image. |
| `apple-touch-icon.png` | 180x180 | The home screen shortcut of iOS. It is padded and drawn on an opaque background, because iOS fills the transparency with black. |
| `icon-192.png` | 192x192 | The web app manifest. |
| `icon-512.png` | 512x512 | The web app manifest. |
| `icon-mask.png` | 512x512 | The maskable icon, which Android crops into the shape of its launcher. Its logo is scaled as large as possible while its visible pixels stay inside the circular safe zone (a square logo gets 288x288 pixels), and it is drawn on an opaque background. |
| `manifest.webmanifest` | - | Web app metadata and icon URLs. |

The tags which belong in the `<head>` element of your pages are printed to the standard output.

```html
<link rel="icon" href="/favicon.ico" sizes="32x32">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="manifest" href="/manifest.webmanifest">
```

By default, `--fit contain` keeps the whole input image and centers it without changing its aspect ratio. ICO and manifest PNG icons use transparent padding; the Apple touch and maskable icons use `--background-color` for their padding. Use `--fit cover` to crop a non-square input from its center instead.

An SVG input is copied as it is, so `--fit` only affects its ICO and PNG outputs. A raster input should be at least 512x512 pixels. A smaller one is enlarged and can look blurry, so a warning is printed to the standard error.

All images and the manifest are generated in memory first. They are written into temporary files next to the output files, which replace the existing files only after all of them are written, so a conversion or write error leaves the existing files unchanged.

## Examples

Generate icons while keeping the whole logo:

```sh
favicon-generator logo.png public/
```

Crop a non-square image from its center instead:

```sh
favicon-generator logo.png public/ --fit cover
```

For a page at `/app/index.html`, put the icons in `/app/static/` and save the HTML tags to a file:

```sh
favicon-generator logo.svg public/app/static/ --path-prefix static/ --start-url /app/ --app-name "My App" --overwrite > favicon-head.html
```

Copy the saved tags into the page's `<head>`. `--path-prefix` applies to these HTML tags. Icon URLs in `manifest.webmanifest` are relative to the manifest's own directory, so `icon-192.png` resolves to `/app/static/icon-192.png` in this example.

Use `--overwrite` in scripts to skip confirmation. The overwrite prompt lists the existing files and goes to the standard error, so it does not enter the saved HTML. Declining the prompt or reaching the end of the input exits with a nonzero status and leaves existing output files unchanged.

## Help

```
EXAMPLES:
favicon-generator /path/to/image /path/to/folder   # Uses /path/to/image to generate favicons into /path/to/folder

Usage: favicon-generator [OPTIONS] <INPUT_PATH> <OUTPUT_PATH>

Arguments:
  <INPUT_PATH>   Assign an image for generating favicons. It should be a path of a file
  <OUTPUT_PATH>  Assign a destination of your generated files. It should be a path of a directory

Options:
  -y, --overwrite                            Overwrite existing files without asking
      --path-prefix <PATH_PREFIX>            Specify the path prefix used by the HTML tags [default: /]
      --fit <FIT>                            Fit the whole image with padding, or crop it from the center [default: contain] [possible values: contain, cover]
      --no-sharpen                           Disable the automatic sharpening
      --app-name <APP_NAME>                  Assign a name for your web app [default: App]
      --app-short-name <APP_SHORT_NAME>      Assign a short name for your web app
      --start-url <START_URL>                Assign the URL your web app starts at [default: /]
      --display <DISPLAY>                    Assign how much browser UI your web app keeps [default: standalone] [possible values: fullscreen, standalone, minimal-ui, browser]
      --background-color <BACKGROUND_COLOR>  Assign the background color of the Apple touch icon and the maskable icon [default: #ffffff]
      --theme-color <THEME_COLOR>            Assign the color the browser UI is tinted with
  -h, --help                                 Print help
  -V, --version                              Print version
```

## License

[MIT](LICENSE)
