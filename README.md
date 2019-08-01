Favicon Generator
====================

[![Build Status](https://travis-ci.org/magiclen/favicon-generator.svg?branch=master)](https://travis-ci.org/magiclen/favicon-generator)

It helps you generate favicons with different formats and sizes.

## Help

```
EXAMPLES:
  favicon-generator /path/to/image /path/to/folder     # Use /path/to/image to generate favicons into /path/to/folder

USAGE:
    favicon-generator [FLAGS] [OPTIONS] <INPUT_PATH> <OUTPUT_PATH>

FLAGS:
    -y, --overwrite     Overwrites exiting files without asking
        --no-sharpen    Disables the automatic sharpening
    -h, --help          Prints help information
    -V, --version       Prints version information

OPTIONS:
        --potrace-path <POTRACE_PATH>             Specifies the path of your potrace executable binary file [default: potrace]
        --path-prefix <PATH_PREFIX>               Specifies the path prefix of your favicon files [default: /]
    -t, --threshold <FLOATING_VALUE>              The black/white cutoff in input file [default: 0.5]
        --app-name <NAME>                         Assigns a name for your web app
        --app-short-name <NAME>                   Assigns a short name for your web app
        --background-color <HEX_COLOR>            Forces to assign a background color for all devices [aliases: background]
        --android-background-color <HEX_COLOR>    Assigns a background color for Android devices [default: #ffffff]  [aliases: android-background]
        --ios-background-color <HEX_COLOR>        Assigns a background color for iOS devices [default: #ffffff]  [aliases: ios-background]
        --safari-background-color <HEX_COLOR>     Assigns a background color for Safari [default: #000000]  [aliases: safari-background]
        --windows-background-color <HEX_COLOR>    Assigns a background color for Windows devices [default: #ffffff]  [aliases: windows-background]

ARGS:
    <INPUT_PATH>     Assigns an image for generating favicons. It should be a path of a file
    <OUTPUT_PATH>    Assigns a destination of your generated files. It should be a path of a directory
```

## License

[MIT](LICENSE)