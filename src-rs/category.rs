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
    #[strum(props(Description = "total lines"))]
    Lines,
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

    pub fn from_query(query: &HashMap<String, Cow<str>>) -> Result<Self, &'static str> {
        match query.get("category") {
            Some(format) => Self::from_str(format).map_err(|_| {
                "Invalid category parameter. Choices are 'code', 'lines', 'files', 'blanks', and \
                 'comments'"
            }),
            None => Ok(Self::Lines),
        }
    }

    pub fn stats(&self, language: &Language) -> usize {
        match self {
            Self::Blanks => language.blanks,
            Self::Files => calc_files(language),
            Self::Comments => language.comments,
            Self::Lines => language.lines(),
            Self::Code => language.code,
        }
    }
}

fn calc_files(language: &Language) -> usize {
    language.reports.len() + language.children.values().map(|r| r.len()).sum::<usize>()
}
