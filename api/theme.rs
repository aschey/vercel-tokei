use std::{borrow::Cow, collections::HashMap};

use crate::{color::Color, style::Style};

pub(crate) struct Theme {
    pub(crate) style: Style,
    pub(crate) label_color: Color,
    pub(crate) color: Color,
}

impl Theme {
    pub(crate) fn from_query(query: &HashMap<Cow<str>, Cow<str>>) -> Result<Self, &'static str> {
        let style = Style::from_query(query)?;
        let label_color = Color::from_query(query, "labelColor", Color::Grey);
        let color = Color::from_query(query, "color", Color::Blue);
        Ok(Self {
            style,
            label_color,
            color,
        })
    }
}
