use crate::on_disk::*;
use banyanfs::{
    api::ApiError,
    codec::header::AccessMaskError,
    filesystem::{DriveAccessError, DriveError, DriveLoaderError, OperationError},
    stores::DataStoreError,
};
use std::{
    fmt::Display,
    path::{PathBuf, StripPrefixError},
    string::FromUtf8Error,
};

#[derive(Debug)]
pub enum NativeError {
    Api(ApiError),
    Disk(OnDiskError),
    Store(DataStoreError),
    Drive(DriveError),
    DriveLoader(DriveLoaderError),
    DriveAccess(DriveAccessError),
    AccessMask(AccessMaskError),
    ConfigState(ConfigStateError),
    Operation(OperationError),
    Custom(String),
}
impl std::error::Error for NativeError {}

impl Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeError::Api(err) => f.write_str(&err.to_string()),
            NativeError::Disk(err) => f.write_str(&err.to_string()),
            NativeError::Drive(err) => f.write_str(&err.to_string()),
            NativeError::DriveLoader(err) => f.write_str(&err.to_string()),
            NativeError::DriveAccess(err) => f.write_str(&err.to_string()),
            NativeError::AccessMask(err) => f.write_str(&err.to_string()),
            NativeError::Store(err) => f.write_str(&err.to_string()),
            NativeError::ConfigState(err) => f.write_str(&err.to_string()),
            NativeError::Operation(err) => f.write_str(&err.to_string()),
            NativeError::Custom(err) => f.write_str(err),
        }
    }
}

#[derive(Debug)]
pub enum ConfigStateError {
    ExpectedPath(PathBuf),
    NoKey,
    NoKeySelected,
    NoAccountId,
    MissingKey(String),
    MissingDrive(String),
    LostPath(String),
}

impl Display for ConfigStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigStateError::ExpectedPath(path) => f.write_str(&format!(
                "EXPECTED PATH TO EXIST BUT DIDN'T {}",
                path.display()
            )),
            ConfigStateError::NoKey => f.write_str("Please create a user key first."),
            ConfigStateError::NoKeySelected => f.write_str("Please select a user key first."),
            ConfigStateError::NoAccountId => f.write_str("Please log in first."),
            ConfigStateError::MissingKey(name) => f.write_str(&format!(
                "Key with name `{}` is not persisted locally.",
                name
            )),
            ConfigStateError::MissingDrive(id) => {
                f.write_str(&format!("MISSING DRIVE WITH ID {}", id))
            }
            ConfigStateError::LostPath(id) => {
                f.write_str(&format!("UNKNOWN PATH OF DRIVE W ID {}", id))
            }
        }
    }
}

impl From<ApiError> for NativeError {
    fn from(value: ApiError) -> Self {
        Self::Api(value)
    }
}

impl From<OnDiskError> for NativeError {
    fn from(value: OnDiskError) -> Self {
        Self::Disk(value)
    }
}

impl From<ConfigStateError> for NativeError {
    fn from(value: ConfigStateError) -> Self {
        Self::ConfigState(value)
    }
}

impl From<std::io::Error> for NativeError {
    fn from(value: std::io::Error) -> Self {
        Self::Disk(OnDiskError::Disk(value))
    }
}

impl From<OperationError> for NativeError {
    fn from(value: OperationError) -> Self {
        Self::Operation(value)
    }
}

impl From<FromUtf8Error> for NativeError {
    fn from(value: FromUtf8Error) -> Self {
        Self::Custom(format!("From UTF8: {value}"))
    }
}

impl From<StripPrefixError> for NativeError {
    fn from(value: StripPrefixError) -> Self {
        Self::Custom(format!("Strip Prefix: {value}"))
    }
}

impl From<uuid::Error> for NativeError {
    fn from(value: uuid::Error) -> Self {
        Self::Custom(format!("UUID parsing: {value}"))
    }
}
impl From<DataStoreError> for NativeError {
    fn from(value: DataStoreError) -> Self {
        Self::Store(value)
    }
}

impl From<DriveError> for NativeError {
    fn from(value: DriveError) -> Self {
        Self::Drive(value)
    }
}

impl From<DriveLoaderError> for NativeError {
    fn from(value: DriveLoaderError) -> Self {
        Self::DriveLoader(value)
    }
}

impl From<DriveAccessError> for NativeError {
    fn from(value: DriveAccessError) -> Self {
        Self::DriveAccess(value)
    }
}

impl From<AccessMaskError> for NativeError {
    fn from(value: AccessMaskError) -> Self {
        Self::AccessMask(value)
    }
}
