//! Management of colored text as configuration.

use core::str::FromStr;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;

use colored::Colorize;
use itertools::join;
use serde::Deserialize;
use serde::Serialize;
use serde::ser::SerializeSeq;

/// Color configuration.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct Color {
    /// Color.
    color: Option<colored::Color>,
}

impl From<colored::Color> for Color {
    fn from(color: colored::Color) -> Self {
        Self { color: Some(color) }
    }
}

impl FromStr for Color {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(colored::Color::from_str(s)?))
    }
}

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        Self::from(colored::Color::AnsiColor(value))
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::from(colored::Color::TrueColor { r, g, b })
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ColorVisitor;

        impl<'de> serde::de::Visitor<'de> for ColorVisitor {
            type Value = Color;

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Color::from_str(value).map_err(|()| {
                    E::custom(format!("Invalid color string: {value}"))
                })
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                u8::try_from(value)
                    .map_err(|_| {
                        E::custom(format!(
                            "ANSI Color value out of range: {value}"
                        ))
                    })
                    .map(Color::from)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut component = |index| {
                    seq.next_element::<u8>()?.ok_or_else(|| {
                        serde::de::Error::invalid_length(index, &self)
                    })
                };
                let color =
                    Color::from((component(0)?, component(1)?, component(2)?));
                if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(serde::de::Error::invalid_length(4, &self));
                }
                Ok(color)
            }

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str(
                    "a string, an integer or an array of 3 elements \
                     representing a color",
                )
            }
        }

        deserializer.deserialize_any(ColorVisitor)
    }
}

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.color {
            Some(c) => match c {
                colored::Color::Black => serializer.serialize_str("black"),
                colored::Color::Red => serializer.serialize_str("red"),
                colored::Color::Green => serializer.serialize_str("green"),
                colored::Color::Yellow => serializer.serialize_str("yellow"),
                colored::Color::Blue => serializer.serialize_str("blue"),
                colored::Color::Magenta => serializer.serialize_str("magenta"),
                colored::Color::Cyan => serializer.serialize_str("cyan"),
                colored::Color::White => serializer.serialize_str("white"),
                colored::Color::BrightBlack => {
                    serializer.serialize_str("bright black")
                }
                colored::Color::BrightRed => {
                    serializer.serialize_str("bright red")
                }
                colored::Color::BrightGreen => {
                    serializer.serialize_str("bright green")
                }
                colored::Color::BrightYellow => {
                    serializer.serialize_str("bright yellow")
                }
                colored::Color::BrightBlue => {
                    serializer.serialize_str("bright blue")
                }
                colored::Color::BrightMagenta => {
                    serializer.serialize_str("bright magenta")
                }
                colored::Color::BrightCyan => {
                    serializer.serialize_str("bright cyan")
                }
                colored::Color::BrightWhite => {
                    serializer.serialize_str("bright white")
                }
                colored::Color::AnsiColor(n) => {
                    serializer.serialize_i64(n.into())
                }
                colored::Color::TrueColor { r, g, b } => {
                    let mut seq = serializer.serialize_seq(Some(3))?;
                    seq.serialize_element(&r)?;
                    seq.serialize_element(&g)?;
                    seq.serialize_element(&b)?;
                    seq.end()
                }
            },
            None => serializer.serialize_none(),
        }
    }
}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(match self.color {
            Some(c) => match c {
                colored::Color::Black => 0,
                colored::Color::Red => 1,
                colored::Color::Green => 2,
                colored::Color::Yellow => 3,
                colored::Color::Blue => 4,
                colored::Color::Magenta => 5,
                colored::Color::Cyan => 6,
                colored::Color::White => 7,
                colored::Color::BrightBlack => 8,
                colored::Color::BrightRed => 9,
                colored::Color::BrightGreen => 10,
                colored::Color::BrightYellow => 11,
                colored::Color::BrightBlue => 12,
                colored::Color::BrightMagenta => 13,
                colored::Color::BrightCyan => 14,
                colored::Color::BrightWhite => 15,
                colored::Color::AnsiColor(n) => 16 + u32::from(n),
                colored::Color::TrueColor { r, g, b } => {
                    17 + u32::from(u8::MAX)
                        + u32::from(r)
                        + u32::from(g)
                        + u32::from(b)
                }
            },
            None => u32::MAX,
        });
    }
}

#[allow(clippy::missing_docs_in_private_items)]
impl Color {
    pub fn none() -> Self {
        Self { color: None }
    }

    pub fn black() -> Self {
        colored::Color::Black.into()
    }

    pub fn red() -> Self {
        colored::Color::Red.into()
    }

    pub fn green() -> Self {
        colored::Color::Green.into()
    }

    pub fn yellow() -> Self {
        colored::Color::Yellow.into()
    }

    pub fn blue() -> Self {
        colored::Color::Blue.into()
    }

    pub fn magenta() -> Self {
        colored::Color::Magenta.into()
    }

    pub fn cyan() -> Self {
        colored::Color::Cyan.into()
    }

    pub fn white() -> Self {
        colored::Color::White.into()
    }

    pub fn bright_black() -> Self {
        colored::Color::BrightBlack.into()
    }

    pub fn bright_red() -> Self {
        colored::Color::BrightRed.into()
    }

    pub fn bright_green() -> Self {
        colored::Color::BrightGreen.into()
    }

    pub fn bright_yellow() -> Self {
        colored::Color::BrightYellow.into()
    }

    pub fn bright_blue() -> Self {
        colored::Color::BrightBlue.into()
    }

    pub fn bright_magenta() -> Self {
        colored::Color::BrightMagenta.into()
    }

    pub fn bright_cyan() -> Self {
        colored::Color::BrightCyan.into()
    }

    pub fn bright_white() -> Self {
        colored::Color::BrightWhite.into()
    }

    pub fn ansi_color(n: u8) -> Self {
        colored::Color::AnsiColor(n).into()
    }

    pub fn true_color(r: u8, g: u8, b: u8) -> Self {
        colored::Color::TrueColor { r, g, b }.into()
    }
}

impl Color {
    /// Colorize the provided text. An empty text stays empty, without color
    /// escape sequences, so that it can be detected as empty.
    pub fn colorize<T>(&self, text: &T) -> String
    where
        T: ToString + ?Sized,
    {
        let text = text.to_string();
        if let Some(color) = self.color
            && !text.is_empty()
        {
            text.color(color).to_string()
        } else {
            text
        }
    }
}

/// Configuration for a colored text.
#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ColoredText {
    /// Text value.
    text: String,
    /// Color of the text.
    pub color: Color,
}

impl ColoredText {
    /// Create a new `ColoredText`.
    pub fn new<S, C>(text: &S, color: C) -> Self
    where
        S: ToString + ?Sized,
        Color: From<C>,
    {
        Self {
            text: text.to_string(),
            color: Color::from(color),
        }
    }

    /// Find out if the colored text is to be considered empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl Display for ColoredText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.color.colorize(&self.text))
    }
}

/// Configuration to display with colors a list.
#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct ColoredList {
    /// Prefix to write in front of the list when displaying it.
    prefix: String,
    /// Separator text to write in-between two items of the list.
    separator: String,
    /// Color to use to print the list (including the prefix and separators).
    color: Color,
}

impl ColoredList {
    /// Create a new `ColoredList`.
    pub fn new<S, C>(prefix: &S, separator: &S, color: C) -> Self
    where
        S: ToString + ?Sized,
        Color: From<C>,
    {
        Self {
            prefix: prefix.to_string(),
            separator: separator.to_string(),
            color: Color::from(color),
        }
    }

    /// Obtain a displayable struct for your list, which will respect the
    /// associated configuration.
    pub fn display<'config, 'list, T>(
        &'config self,
        list: &'list [T],
    ) -> ColoredListDisplay<'config, 'list, T>
    where
        T: ToString,
    {
        ColoredListDisplay {
            colored_list: self,
            list,
        }
    }
}

/// Displayable struct for `ColoredList`.
pub struct ColoredListDisplay<'config, 'list, T> {
    /// Configuration to use to display the list.
    colored_list: &'config ColoredList,
    /// List to display.
    list: &'list [T],
}

impl<T> Display for ColoredListDisplay<'_, '_, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.list.is_empty() {
            return Ok(());
        }

        write!(
            f,
            "{}",
            self.colored_list.color.colorize(&format!(
                "{}{}",
                self.colored_list.prefix,
                join(self.list.iter(), &self.colored_list.separator)
            ))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::colors;
    use crate::test_utils::no_colors;

    /// Wrapper to deserialize a color from a TOML value.
    #[derive(Debug, Deserialize, Serialize)]
    struct Wrapper {
        /// The color.
        color: Color,
    }

    /// Parse `value` as the TOML value of a color.
    fn parse(value: &str) -> Result<Color, toml::de::Error> {
        toml::from_str::<Wrapper>(&format!("color = {value}")).map(|w| w.color)
    }

    #[test]
    fn color_from_name() {
        assert_eq!(parse("'red'").expect("valid color"), Color::red());
        assert_eq!(
            parse("'bright blue'").expect("valid color"),
            Color::bright_blue()
        );
    }

    #[test]
    fn color_from_unknown_name_fails() {
        let error = parse("'reddish'").expect_err("invalid color");

        assert!(error.to_string().contains("Invalid color string: reddish"));
    }

    #[test]
    fn color_from_ansi_number() {
        assert_eq!(parse("0").expect("valid color"), Color::ansi_color(0));
        assert_eq!(parse("255").expect("valid color"), Color::ansi_color(255));
    }

    #[test]
    fn color_from_out_of_range_number_fails() {
        for value in ["256", "-1"] {
            let error = parse(value).expect_err("invalid color");

            assert!(
                error.to_string().contains("ANSI Color value out of range"),
                "{error}"
            );
        }
    }

    #[test]
    fn color_from_rgb_array() {
        assert_eq!(
            parse("[1, 2, 3]").expect("valid color"),
            Color::true_color(1, 2, 3)
        );
    }

    #[test]
    fn color_from_array_of_wrong_length_fails() {
        for value in ["[]", "[1, 2]", "[1, 2, 3, 4]"] {
            let error = parse(value).expect_err("invalid color");

            assert!(
                error.to_string().contains("array of 3 elements"),
                "{value}: {error}"
            );
        }
    }

    #[test]
    fn color_from_array_out_of_range_fails() {
        assert!(parse("[1, 2, 300]").is_err());
    }

    #[test]
    fn color_from_other_type_fails() {
        assert!(parse("true").is_err());
    }

    #[test]
    fn color_serialization_round_trips() {
        for color in [
            Color::black(),
            Color::bright_white(),
            Color::bright_black(),
            Color::ansi_color(42),
            Color::true_color(10, 20, 30),
        ] {
            let text = toml::to_string(&Wrapper {
                color: color.clone(),
            })
            .expect("serializable color");

            assert_eq!(
                toml::from_str::<Wrapper>(&text).expect("valid color").color,
                color,
                "{text}"
            );
        }
    }

    #[test]
    fn colored_text_fields_are_optional() {
        let text: ColoredText =
            toml::from_str("text = 'x'").expect("valid colored text");
        assert_eq!(text, ColoredText::new("x", Color::none()));

        let text: ColoredText =
            toml::from_str("color = 'red'").expect("valid colored text");
        assert_eq!(text, ColoredText::new("", Color::red()));
        assert!(text.is_empty());
    }

    #[test]
    fn colored_text_without_color_round_trips() {
        let text = ColoredText::new("x", Color::none());
        let serialized = toml::to_string(&text).expect("serializable text");

        assert_eq!(
            toml::from_str::<ColoredText>(&serialized).expect("valid text"),
            text
        );
    }

    #[test]
    fn colorize_without_color_keeps_text() {
        let _colors = colors();

        assert_eq!(Color::none().colorize("text"), "text");
    }

    #[test]
    fn colorize_applies_the_color() {
        let _colors = colors();

        assert_eq!(Color::red().colorize("text"), "\x1b[31mtext\x1b[0m");
        // True colors are not checked: `colored` downgrades them to the
        // closest basic color unless `COLORTERM` announces true color support.
        assert_eq!(
            Color::ansi_color(42).colorize("text"),
            "\x1b[38;5;42mtext\x1b[0m"
        );
    }

    #[test]
    fn colorize_empty_text_stays_empty() {
        let _colors = colors();

        assert_eq!(Color::red().colorize(""), "");
    }

    #[test]
    fn colorize_respects_disabled_colors() {
        let _colors = no_colors();

        assert_eq!(Color::red().colorize("text"), "text");
    }

    #[test]
    fn colored_list_display() {
        let _colors = no_colors();
        let list = ColoredList::new("<", ", ", Color::red());

        assert_eq!(list.display::<&str>(&[]).to_string(), "");
        assert_eq!(list.display(&["a"]).to_string(), "<a");
        assert_eq!(list.display(&["a", "b", "c"]).to_string(), "<a, b, c");
    }

    #[test]
    fn empty_colored_list_has_no_color_codes() {
        let _colors = colors();
        let list = ColoredList::new("<", ", ", Color::red());

        assert_eq!(list.display::<&str>(&[]).to_string(), "");
    }
}
