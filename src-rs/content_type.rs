use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;

use strum::EnumProperty;
use strum_macros::{EnumProperty, EnumString};

#[derive(PartialEq, Eq, Debug, EnumString, EnumProperty)]
#[strum(ascii_case_insensitive)]
pub enum ContentType {
    #[strum(props(ResponseType = "image/svg+xml"))]
    Svg,
    #[strum(props(ResponseType = "application/json"))]
    Json,
}

impl ContentType {
    pub fn from_query(query: &HashMap<String, Cow<str>>) -> Result<Self, &'static str> {
        match query.get("format") {
            Some(format) => Self::from_str(format)
                .map_err(|_| "Invalid format parameter. Choices are 'svg' and 'json'"),
            None => Ok(Self::Svg),
        }
    }

    pub fn response_type(&self) -> &str {
        self.get_str("ResponseType")
            .expect("ResponseType should be set")
    }
}
