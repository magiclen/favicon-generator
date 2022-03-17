Favicon Generator
====================

[![CI](https://github.com/magiclen/favicon-generator/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/favicon-generator/actions/workflows/ci.yml)

It helps you generate favicons with different formats and sizes.

## Help

```
EXAMPLES:
favicon-generator /path/to/image /path/to/folder     # Uses /path/to/image to generate favicons into /path/to/folder

USAGE:
    favicon-generator [OPTIONS] <INPUT_PATH> <OUTPUT_PATH>

ARGS:
    <INPUT_PATH>     Assign an image for generating favicons. It should be a path of a file
    <OUTPUT_PATH>    Assign a destination of your generated files. It should be a path of a directory

OPTIONS:
    -y, --overwrite                               Overwrite exiting files without asking
        --potrace-path <POTRACE_PATH>             Specify the path of your potrace executable binary file [default: potrace]
        --path-prefix <PATH_PREFIX>               Specify the path prefix of your favicon files [default: /]
    -t, --threshold <FLOATING_VALUE>              The black/white cutoff in input file [default: 0.5]
        --no-sharpen                              Disable the automatic sharpening
        --app-name <NAME>                         Assign a name for your web app
        --app-short-name <NAME>                   Assign a short name for your web app
        --background-color <HEX_COLOR>            Force to assign a background color for all devices [aliases: background]
        --android-background-color <HEX_COLOR>    Assign a background color for Android devices [default: #ffffff] [aliases: android-background]
        --ios-background-color <HEX_COLOR>        Assign a background color for iOS devices [default: #ffffff] [aliases: ios-background]
        --safari-background-color <HEX_COLOR>     Assign a background color for Safari [default: #000000] [aliases: safari-background]
        --windows-background-color <HEX_COLOR>    Assign a background color for Windows devices [default: #ffffff] [aliases: windows-background]
    -h, --help                                    Print help information
    -V, --version                                 Print version information
```

## License

[MIT](LICENSE)