use std::fmt::Write;

use crate::{
    cli::CLIArgs,
    favicon::{
        FILE_APPLE_TOUCH_ICON, FILE_FAVICON_ICO, FILE_FAVICON_SVG, FILE_WEB_APP_MANIFEST, ICO_SIZE,
    },
};

/// Build the tags which belong in the `<head>` element of a page.
///
/// Every value is escaped by hand, because these tags are not built from a template engine which would do it.
pub fn build_head(args: &CLIArgs, svg: bool) -> String {
    let path_prefix = html_escape::encode_double_quoted_attribute(args.path_prefix.as_str());

    let mut head = String::new();

    // the `sizes` attribute tells the browser this file holds no bigger image, so a browser which supports the SVG icon does not download it
    writeln!(
        head,
        r#"<link rel="icon" href="{path_prefix}{FILE_FAVICON_ICO}" sizes="{ICO_SIZE}x{ICO_SIZE}">"#
    )
    .unwrap();

    if svg {
        writeln!(
            head,
            r#"<link rel="icon" href="{path_prefix}{FILE_FAVICON_SVG}" type="image/svg+xml">"#
        )
        .unwrap();
    }

    writeln!(head, r#"<link rel="apple-touch-icon" href="{path_prefix}{FILE_APPLE_TOUCH_ICON}">"#)
        .unwrap();

    writeln!(head, r#"<link rel="manifest" href="{path_prefix}{FILE_WEB_APP_MANIFEST}">"#).unwrap();

    if let Some(theme_color) = args.theme_color.as_deref() {
        let theme_color = html_escape::encode_double_quoted_attribute(theme_color);

        writeln!(head, r#"<meta name="theme-color" content="{theme_color}">"#).unwrap();
    }

    // the caller prints the result as a line of its own
    head.pop();

    head
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn args_from(extra: &[&str]) -> CLIArgs {
        let mut argv = vec!["favicon-generator", "logo.png", "out"];

        argv.extend_from_slice(extra);

        CLIArgs::parse_from(argv)
    }

    #[test]
    fn head_of_a_raster_input() {
        assert_eq!(
            r#"<link rel="icon" href="/favicon.ico" sizes="32x32">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="manifest" href="/manifest.webmanifest">"#,
            build_head(&args_from(&[]), false)
        );
    }

    #[test]
    fn head_of_a_vector_input() {
        assert_eq!(
            r#"<link rel="icon" href="/favicon.ico" sizes="32x32">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="manifest" href="/manifest.webmanifest">"#,
            build_head(&args_from(&[]), true)
        );
    }

    #[test]
    fn head_with_a_path_prefix_and_a_theme_color() {
        assert_eq!(
            r##"<link rel="icon" href="/static/favicon.ico" sizes="32x32">
<link rel="apple-touch-icon" href="/static/apple-touch-icon.png">
<link rel="manifest" href="/static/manifest.webmanifest">
<meta name="theme-color" content="#1e40af">"##,
            build_head(
                &args_from(&["--path-prefix", "/static", "--theme-color", "#1E40AF"]),
                false
            )
        );
    }
}
