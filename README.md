Favicon Generator
====================

[![CI](https://github.com/magiclen/favicon-generator/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/favicon-generator/actions/workflows/ci.yml)

It helps you generate favicons with different formats and sizes.

## Generated Files

| File | Size | Purpose |
| ---- | ---- | ------- |
| `favicon.ico` | 32x32 | Browsers which do not support the SVG icon, and the automatic `/favicon.ico` request. |
| `favicon.svg` | - | The icon of any size, generated only when the input image is an SVG image. |
| `apple-touch-icon.png` | 180x180 | The home screen shortcut of iOS. It is padded and drawn on an opaque background, because iOS fills the transparency with black. |
| `icon-192.png` | 192x192 | The web app manifest. |
| `icon-512.png` | 512x512 | The web app manifest. |
| `icon-mask.png` | 512x512 | The maskable icon, which Android crops into the shape of its launcher. Its logo stays inside the safe zone, and it is drawn on an opaque background. |
| `manifest.webmanifest` | - | The web app manifest, which fills in every member Chrome asks for before it offers to install a web app. |

The tags which belong in the `<head>` element of your pages are printed to the standard output.

```html
<link rel="icon" href="/favicon.ico" sizes="32x32">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="manifest" href="/manifest.webmanifest">
```

An input image which is not square is cropped into a square from its center. An SVG input image is copied as it is, so it keeps its own aspect ratio.

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
      --path-prefix <PATH_PREFIX>            Specify the path prefix of your favicon files [default: /]
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
