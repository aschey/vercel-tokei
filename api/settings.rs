use std::{borrow::Cow, collections::HashMap};

use crate::{content_type::ContentType, theme::Theme, Category};

const DEFAULT_CACHE_SECONDS: u32 = 60;

pub(crate) struct Settings {
    pub(crate) category: Category,
    pub(crate) content_type: ContentType,
    pub(crate) theme: Theme,
    pub(crate) cache_seconds: u32,
}

impl Settings {
    pub(crate) fn from_query(query: &HashMap<Cow<str>, Cow<str>>) -> Result<Self, &'static str> {
        let category = Category::from_query(query)?;
        let content_type = ContentType::from_query(query)?;
        let theme = Theme::from_query(query)?;

        let cache_seconds: u32 = match query.get("cacheSeconds") {
            Some(seconds) => seconds.parse().unwrap_or(DEFAULT_CACHE_SECONDS),
            None => DEFAULT_CACHE_SECONDS,
        };

        Ok(Self {
            cache_seconds,
            category,
            theme,
            content_type,
        })
    }
}
