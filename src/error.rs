use std::{fmt::Display, path::PathBuf, string::FromUtf8Error};

use banyanfs::{
    api::{ApiClientError, ApiError},
    error::BanyanFsError,
};
use colored::Colorize;
use uuid::Uuid;

//#[cfg(feature = "cli")]
//use {crate::cli::specifiers::DriveSpecifier, std::path::PathBuf, uuid::Uuid};

#[derive(Debug)]
pub struct NativeError {
    kind: NativeErrorKind,
}

impl std::error::Error for NativeError {}

impl Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            NativeErrorKind::MissingApiKey => "Unable to find API Key".to_owned(),
            NativeErrorKind::MissingWrappingKey => "Unable to find Wrapping Key".to_owned(),
            NativeErrorKind::MissingUserId => "Unable to find remote User Id".to_owned(),
            NativeErrorKind::MissingIdentifier => {
                "Unable to find a remote Identifier associated with that Drive".to_owned()
            }
            NativeErrorKind::MissingLocalDrive => {
                "Unable to find a local Drive with that query".to_owned()
            }
            NativeErrorKind::MissingRemoteDrive => {
                "Unable to find a remote Drive with that query".to_owned()
            }
            NativeErrorKind::UniqueDriveError => {
                "There is already a unique Drive with these specs".to_owned()
            }
            NativeErrorKind::BadData => "bad data".to_owned(),
            NativeErrorKind::Custom(msg) => msg.to_owned(),
            NativeErrorKind::Filesystem(err) => {
                format!("{} {err}", "FILESYSTEM ERROR:".underline())
            }
            NativeErrorKind::Api(err) => format!("{} {err}", "CLIENT ERROR:".underline()),
            NativeErrorKind::Io(err) => format!("{} {err}", "IO ERROR:".underline()),
            NativeErrorKind::UnknownDriveId(id) => format!("No known Drive with id {id}"),
            NativeErrorKind::UnknownDrivePath(path) => {
                format!("No known Drive with path {}", path.display())
            }
        };

        f.write_str(&string)
    }
}

impl NativeError {
    pub fn missing_api_key() -> Self {
        Self {
            kind: NativeErrorKind::MissingApiKey,
        }
    }

    pub fn missing_wrapping_key() -> Self {
        Self {
            kind: NativeErrorKind::MissingWrappingKey,
        }
    }

    pub fn missing_user_id() -> Self {
        Self {
            kind: NativeErrorKind::MissingUserId,
        }
    }

    pub fn missing_identifier() -> Self {
        Self {
            kind: NativeErrorKind::MissingIdentifier,
        }
    }

    pub fn missing_local_drive() -> Self {
        Self {
            kind: NativeErrorKind::MissingLocalDrive,
        }
    }

    pub fn missing_remote_drive() -> Self {
        Self {
            kind: NativeErrorKind::MissingRemoteDrive,
        }
    }

    pub fn unique_error() -> Self {
        Self {
            kind: NativeErrorKind::UniqueDriveError,
        }
    }

    pub fn bad_data() -> Self {
        Self {
            kind: NativeErrorKind::BadData,
        }
    }

    pub fn custom_error(msg: &str) -> Self {
        Self {
            kind: NativeErrorKind::Custom(msg.to_owned()),
        }
    }

    pub fn filesystem(err: BanyanFsError) -> Self {
        Self {
            kind: NativeErrorKind::Filesystem(err),
        }
    }

    pub fn api(err: ApiError) -> Self {
        Self {
            kind: NativeErrorKind::Api(err),
        }
    }

    pub fn io(err: std::io::Error) -> Self {
        Self {
            kind: NativeErrorKind::Io(err),
        }
    }

    /// Unknown Bucket path
    pub fn unknown_path(path: PathBuf) -> Self {
        Self {
            kind: NativeErrorKind::UnknownDrivePath(path),
        }
    }

    /// Unknown Bucket ID
    pub fn unknown_id(id: Uuid) -> Self {
        Self {
            kind: NativeErrorKind::UnknownDriveId(id),
        }
    }
}

#[derive(Debug)]
enum NativeErrorKind {
    MissingApiKey,
    MissingWrappingKey,
    MissingUserId,
    MissingIdentifier,
    MissingLocalDrive,
    MissingRemoteDrive,
    UniqueDriveError,
    BadData,
    Custom(String),
    Filesystem(BanyanFsError),
    Api(ApiError),
    Io(std::io::Error),
    UnknownDrivePath(PathBuf),
    UnknownDriveId(Uuid),
}

impl From<BanyanFsError> for NativeError {
    fn from(value: BanyanFsError) -> Self {
        Self::filesystem(value)
    }
}

impl From<ApiError> for NativeError {
    fn from(value: ApiError) -> Self {
        Self::api(value)
    }
}

impl From<std::io::Error> for NativeError {
    fn from(value: std::io::Error) -> Self {
        Self::io(value)
    }
}

impl From<FromUtf8Error> for NativeError {
    fn from(value: FromUtf8Error) -> Self {
        Self::custom_error(&format!("From UTF8: {value}"))
    }
}
