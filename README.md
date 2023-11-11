Favicon Generator
====================

[![CI](https://github.com/magiclen/favicon-generator/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/favicon-generator/actions/workflows/ci.yml)

It helps you generate favicons with different formats and sizes.

## Help

```
EXAMPLES:
favicon-generator /path/to/image /path/to/folder   # Uses /path/to/image to generate favicons into /path/to/folder

Usage: favicon-generator [OPTIONS] <INPUT_PATH> <OUTPUT_PATH>

Arguments:
  <INPUT_PATH>   Assign an image for generating favicons. It should be a path of a file
  <OUTPUT_PATH>  Assign a destination of your generated files. It should be a path of a directory

Options:
  -y, --overwrite                        Overwrite exiting files without asking
      --path-prefix <PATH_PREFIX>        Specify the path prefix of your favicon files [default: /]
      --no-sharpen                       Disable the automatic sharpening
      --app-name <APP_NAME>              Assign a name for your web app [default: App]
      --app-short-name <APP_SHORT_NAME>  Assign a short name for your web app
  -h, --help                             Print help
  -V, --version                          Print version
```

## License

[MIT](LICENSE)