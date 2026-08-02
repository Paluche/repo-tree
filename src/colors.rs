//! Management of colored text as configuration.

use core::str::FromStr;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;

use colored::Colorize;
use itertools::join;
use serde::Deserialize;
use serde::Serialize;
use serde::ser::SerializeSeq;

/// Trait to implement is_empty().
pub trait IsEmpty {
    /// Find out if the struct is to be considered empty.
    fn is_empty(&self) -> bool;
}

impl<T> IsEmpty for T
where
    T: Deref<Target = str>,
{
    fn is_empty(&self) -> bool {
        self.deref().is_empty()
    }
}

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
                Color::from_str(value).map_err(|_| {
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
                Ok(Color::from((
                    seq.next_element()?.unwrap(),
                    seq.next_element()?.unwrap(),
                    seq.next_element()?.unwrap(),
                )))
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
                colored::Color::AnsiColor(n) => 16 + n as u32,
                colored::Color::TrueColor { r, g, b } => {
                    17 + u8::MAX as u32 + r as u32 + g as u32 + b as u32
                }
            },
            None => u32::MAX,
        });
    }
}

impl Color {
    /// Colorize the provided text.
    pub fn colorize<T>(&self, text: T) -> String
    where
        T: ToString,
    {
        if let Some(color) = self.color {
            text.to_string().color(color).to_string()
        } else {
            text.to_string()
        }
    }
}

/// Configuration for a colored text.
#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct ColoredText {
    /// Text value.
    text: String,
    /// Color of the text.
    color: Color,
}

impl Deref for ColoredText {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

impl IsEmpty for &ColoredText {
    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl ColoredText {
    /// Create a new ColoredText.
    pub fn new<S, C>(text: S, color: C) -> Self
    where
        S: ToString,
        Color: From<C>,
    {
        Self {
            text: text.to_string(),
            color: Color::from(color),
        }
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
    /// Create a new ColoredList.
    pub fn new<S, C>(prefix: S, separator: S, color: C) -> Self
    where
        S: ToString,
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

/// Displayable struct for ColoredList.
pub struct ColoredListDisplay<'config, 'list, T> {
    /// Configuration to use to display the list.
    colored_list: &'config ColoredList,
    /// List to display.
    list: &'list [T],
}

impl<'config, 'list, T> IsEmpty for ColoredListDisplay<'config, 'list, T> {
    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl<'config, 'list, T> Display for ColoredListDisplay<'config, 'list, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.colored_list.color.colorize(format!(
                "{}{}",
                self.colored_list.prefix,
                join(self.list.iter(), &self.colored_list.separator)
            ))
        )
    }
}
