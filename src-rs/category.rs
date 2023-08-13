use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;

use strum::EnumProperty;
use strum_macros::{EnumProperty, EnumString};
use tokei::Language;

#[derive(PartialEq, Eq, Debug, EnumString, EnumProperty)]
#[strum(ascii_case_insensitive)]
pub enum Category {
    #[strum(props(Description = "blank lines"))]
    Blanks,
    #[strum(props(Description = "lines of code"))]
    Code,
    #[strum(props(Description = "comments"))]
    Comments,
    #[strum(props(Description = "files"))]
    Files,
}

impl Category {
    pub fn description(&self) -> &str {
        self.get_str("Description")
            .expect("description should be set")
    }

    pub fn from_query(query: &HashMap<Cow<str>, Cow<str>>) -> Result<Self, &'static str> {
        match query.get("category") {
            Some(format) => Self::from_str(format).map_err(|_| {
                "Invalid category parameter. Choices are 'code', 'files', 'blanks', and 'comments'"
            }),
            None => Ok(Self::Code),
        }
    }

    pub fn stats(&self, language: &Language) -> usize {
        match self {
            Self::Blanks => language.blanks,
            Self::Files => language.reports.len(),
            Self::Comments => language.comments,
            Self::Code => language.code,
        }
    }
}
