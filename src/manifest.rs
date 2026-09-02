use clap::ValueEnum;
use serde::Serialize;

use crate::{
    cli::CLIArgs,
    favicon::{
        FILE_MASKABLE_ICON, MANIFEST_ICON_SIZES, MASKABLE_ICON_SIZE, manifest_icon_file_name,
    },
};

const ICON_MIME_TYPE: &str = "image/png";

/// The purpose of an icon which the launcher of the platform may crop into its own shape.
const MASKABLE_PURPOSE: &str = "maskable";

/// The `display` member of a web app manifest, which tells the browser how much of its own interface to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayMode {
    Fullscreen,
    Standalone,
    MinimalUi,
    Browser,
}

#[derive(Debug, Serialize)]
pub struct Icon {
    pub src:     String,
    pub sizes:   String,
    #[serde(rename = "type")]
    pub mime:    &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct WebAppManifest<'a> {
    pub name:             &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name:       Option<&'a str>,
    pub start_url:        &'a str,
    pub display:          DisplayMode,
    pub background_color: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_color:      Option<&'a str>,
    pub icons:            Vec<Icon>,
}

impl<'a> WebAppManifest<'a> {
    pub fn new(args: &'a CLIArgs) -> WebAppManifest<'a> {
        let path_prefix = args.path_prefix.as_str();

        let mut icons: Vec<Icon> = MANIFEST_ICON_SIZES
            .iter()
            .copied()
            .map(|size| Icon {
                src:     format!("{path_prefix}{}", manifest_icon_file_name(size)),
                sizes:   format!("{size}x{size}"),
                mime:    ICON_MIME_TYPE,
                purpose: None,
            })
            .collect();

        icons.push(Icon {
            src:     format!("{path_prefix}{FILE_MASKABLE_ICON}"),
            sizes:   format!("{MASKABLE_ICON_SIZE}x{MASKABLE_ICON_SIZE}"),
            mime:    ICON_MIME_TYPE,
            purpose: Some(MASKABLE_PURPOSE),
        });

        WebAppManifest {
            name: args.app_name.as_str(),
            short_name: args.app_short_name.as_deref(),
            start_url: args.start_url.as_str(),
            display: args.display,
            background_color: args.background_color.as_str(),
            theme_color: args.theme_color.as_deref(),
            icons,
        }
    }

    /// Serialize the manifest. It is pretty-printed, because it is a file which its owner may want to edit afterwards.
    #[inline]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
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
    fn manifest_with_the_default_options() {
        let args = args_from(&[]);

        assert_eq!(
            r##"{
  "name": "App",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "icons": [
    {
      "src": "/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    },
    {
      "src": "/icon-mask.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "maskable"
    }
  ]
}"##,
            WebAppManifest::new(&args).to_json()
        );
    }

    #[test]
    fn manifest_with_every_option() {
        let args = args_from(&[
            "--path-prefix",
            "/static",
            "--app-name",
            "Magic Len",
            "--app-short-name",
            "ML",
            "--start-url",
            "/app",
            "--display",
            "minimal-ui",
            "--background-color",
            "#000",
            "--theme-color",
            "#1E40AF",
        ]);

        assert_eq!(
            r##"{
  "name": "Magic Len",
  "short_name": "ML",
  "start_url": "/app",
  "display": "minimal-ui",
  "background_color": "#000000",
  "theme_color": "#1e40af",
  "icons": [
    {
      "src": "/static/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/static/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    },
    {
      "src": "/static/icon-mask.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "maskable"
    }
  ]
}"##,
            WebAppManifest::new(&args).to_json()
        );
    }
}
