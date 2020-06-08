//! # Favicon Generator
//! It helps you generate favicons with different formats and sizes.

#[macro_use]
extern crate validators;

#[macro_use]
extern crate lazy_static;

use validators::regex::Regex;

lazy_static! {
    static ref RE_HEX_COLOR: Regex = Regex::new("^#[0-f0-F]{6}$").unwrap();
}

validated_customized_ranged_number!(pub Threshold, f64, 0f64, 1.0f64);
validated_customized_regex_string!(pub HexColor, ref RE_HEX_COLOR);
