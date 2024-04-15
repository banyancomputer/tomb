use colored::Colorize;
use std::{fmt::Display, string::FromUtf8Error};

#[derive(Debug)]
pub struct UtilityError {
    kind: UtilityErrorKind,
}

impl UtilityError {
    pub fn custom(msg: &str) -> Self {
        Self {
            kind: UtilityErrorKind::Custom(msg.to_owned()),
        }
    }

    pub fn varint(err: unsigned_varint::decode::Error) -> Self {
        Self {
            kind: UtilityErrorKind::Varint(err),
        }
    }

    pub fn io(err: std::io::Error) -> Self {
        Self {
            kind: UtilityErrorKind::Io(err),
        }
    }

    pub fn utf8(err: FromUtf8Error) -> Self {
        Self {
            kind: UtilityErrorKind::Utf8(err),
        }
    }
}

impl Display for UtilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            UtilityErrorKind::Custom(msg) => msg.to_owned(),
            UtilityErrorKind::Varint(err) => format!("{} {err}", "VARINT ERROR:".underline()),
            UtilityErrorKind::Io(err) => format!("{} {err}", "IO ERROR:".underline()),
            UtilityErrorKind::Utf8(err) => format!("{} {err}", "UTF8 ERROR:".underline()),
            //            UtilityErrorKind::Native(err) => format!("{} {err}", "NATIVE ERROR:".underline()),
        };

        f.write_str(&string)
    }
}

#[derive(Debug)]
pub enum UtilityErrorKind {
    Custom(String),
    Varint(unsigned_varint::decode::Error),
    Io(std::io::Error),
    Utf8(FromUtf8Error),
}

impl From<unsigned_varint::decode::Error> for UtilityError {
    fn from(value: unsigned_varint::decode::Error) -> Self {
        Self::varint(value)
    }
}

impl From<std::io::Error> for UtilityError {
    fn from(value: std::io::Error) -> Self {
        Self::io(value)
    }
}

impl From<FromUtf8Error> for UtilityError {
    fn from(value: FromUtf8Error) -> Self {
        Self::utf8(value)
    }
}
