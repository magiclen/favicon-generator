//! # Favicon Generator
//! It helps you generate favicons with different formats and sizes.

use once_cell::sync::Lazy;
use validators::prelude::*;
use validators_prelude::regex::Regex;

static RE_HEX_COLOR: Lazy<Regex> = Lazy::new(|| Regex::new("^#[0-f0-F]{6}$").unwrap());

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Validator)]
#[validator(number(nan(NotAllow), range(Inside(min = 0, max = 1))))]
pub struct Threshold(f64);

impl Threshold {
    #[inline]
    pub fn get_number(&self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Validator)]
#[validator(regex(RE_HEX_COLOR))]
pub struct HexColor(String);

impl HexColor {
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Validator)]
#[validator(boolean)]
pub struct Boolean(bool);

impl Boolean {
    #[inline]
    pub fn get_bool(&self) -> bool {
        self.0
    }
}
