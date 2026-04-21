// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use thiserror::Error;

use crate::HapticData;
use crate::Version;
use crate::v1;
use crate::v1::ValidationMode;

/// Parses HapticData from a JSON string
pub fn from_json(data: &str, validation_mode: ValidationMode) -> Result<HapticData, FromJsonError> {
    use FromJsonError::*;

    match Version::from_json(data) {
        Ok(Version { major: 1, .. }) => match serde_json::from_str::<v1::HapticData>(data) {
            Ok(deserialized_data) => match deserialized_data.validate(validation_mode) {
                Ok(validated_data) => Ok(validated_data),
                Err(e) => Err(V1ValidationError(e)),
            },
            Err(e) => Err(InvalidJson(e.to_string())),
        },
        Ok(unsupported) => Err(UnsupportedVersion(unsupported)),
        Err(e) => Err(InvalidJson(e.to_string())),
    }
}

/// The different kinds of haptic data validation errors that can occur
#[derive(Error, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum FromJsonError {
    #[error("invalid JSON: {0}")]
    InvalidJson(String),
    #[error("unsupported version: {0}")]
    UnsupportedVersion(Version),
    #[error(transparent)]
    V1ValidationError(v1::ValidationError),
}
