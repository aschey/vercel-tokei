use std::{borrow::Cow, collections::HashMap, str::FromStr};

use rsbadges::Badge;
use strum_macros::EnumString;

#[derive(PartialEq, Eq, Debug, EnumString)]
#[strum(serialize_all = "kebab-case", ascii_case_insensitive)]
pub(crate) enum Style {
    Flat,
    FlatSquare,
    Plastic,
    ForTheBadge,
    Social,
}

impl Style {
    pub(crate) fn from_query(query: &HashMap<Cow<str>, Cow<str>>) -> Result<Self, &'static str> {
        match query.get("style") {
            Some(style) => Self::from_str(style)
                .map_err(|_| "Invalid style parameter. Choices are 'flat', 'flat-square', 'plastic', 'for-the-badge', and 'social'"),
            None => Ok(Self::Flat),
        }
    }

    pub(crate) fn to_badge_style(&self, badge: Badge) -> rsbadges::Style {
        match self {
            Self::Flat => rsbadges::Style::Flat(badge),
            Self::FlatSquare => rsbadges::Style::FlatSquare(badge),
            Self::Plastic => rsbadges::Style::Plastic(badge),
            Self::Social => rsbadges::Style::Social(badge),
            Self::ForTheBadge => rsbadges::Style::ForTheBadge(badge),
        }
    }
}
