// Named colors from https://github.com/badges/shields/blob/7d452472defa0e0bd71d6443393e522e8457f856/badge-maker/lib/color.js

use std::{borrow::Cow, collections::HashMap, str::FromStr};

use strum::EnumProperty;
use strum_macros::{EnumProperty, EnumString};

#[derive(PartialEq, Eq, Debug, EnumString, EnumProperty)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub(crate) enum Color {
    #[strum(props(Hex = "#4c1"))]
    BrightGreen,
    #[strum(props(Hex = "#97ca00"))]
    Green,
    #[strum(props(Hex = "#dfb317"))]
    Yellow,
    #[strum(props(Hex = "#a4a61d"))]
    YellowGreen,
    #[strum(props(Hex = "#fe7d37"))]
    Orange,
    #[strum(props(Hex = "#e05d44"))]
    Red,
    #[strum(props(Hex = "#007ec6"))]
    Blue,
    #[strum(props(Hex = "#555"))]
    Grey,
    #[strum(props(Hex = "#9f9f9f"))]
    LightGrey,
    #[strum(disabled)]
    Other(String),
}

impl Color {
    pub(crate) fn from_query(
        query: &HashMap<Cow<str>, Cow<str>>,
        key: &str,
        default: Color,
    ) -> Self {
        match query.get(key) {
            Some(color) => Self::from_str(color).unwrap_or_else(|_| {
                let mut color = color.to_string();
                let re = lazy_regex::regex!(r"^([\da-f]{3}){1,2}$");
                if re.is_match(&color) {
                    color = format!("#{color}");
                }

                Self::Other(color)
            }),
            None => default,
        }
    }
}

impl ToString for Color {
    fn to_string(&self) -> String {
        match self {
            Self::Other(val) => val.to_owned(),
            _ => self
                .get_str("Hex")
                .expect("Color variant should have hex property")
                .to_owned(),
        }
    }
}
