// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use haptic_data::HapticData;
use haptic_data::ahap::Ahap;
use haptic_data::from_json;
use haptic_data::v1::ValidationMode;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

use crate::helpers::napi_error_from_error;

/// An AHAP that is split into its continuous and transient components.
///
/// Such a split is useful when playing back a haptic clip with the Core Haptics API. If everything
/// is in one AHAP, Core Haptic's CHHapticPatternPlayer modulates the transients based on the
/// contents of the continuous curves. We however want the transients to be played back
/// independently of the continuous curves. This can be achieved by using two different
/// CHHapticPatternPlayer, loading SplitAhap::continuous into one, and SplitAhap::transients
/// into the other.
#[derive(Serialize, Deserialize)]
#[typeshare]
pub struct SplitAhap {
    continuous: String,
    transients: Option<String>,
}

/// Converts a JSON string into HapticData
///
/// An error is thrown if validation of the haptic data fails
#[napi]
pub fn haptic_data_from_json(json: String) -> Result<serde_json::Value> {
    let data =
        from_json(&json, ValidationMode::LegacyCompatibility).map_err(napi_error_from_error)?;

    let data = serde_json::to_value(&data)?;
    Ok(data)
}

/// Converts HapticData to AHAP
#[napi]
pub fn haptic_data_to_ahap(haptic_data: serde_json::Value) -> Result<String> {
    let haptic_data: HapticData = serde_json::from_value(haptic_data)?;
    let ahap: Ahap = haptic_data.into();
    serde_json::to_string(&ahap).map_err(napi_error_from_error)
}

/// Converts HapticData to SplitAhap
#[napi]
pub fn haptic_data_to_split_ahap(haptic_data: serde_json::Value) -> Result<serde_json::Value> {
    let haptic_data: HapticData = serde_json::from_value(haptic_data)?;
    let ahap: Ahap = haptic_data.into();
    let split_ahap: haptic_data::ahap::SplitAhap = ahap.into();

    let continuous =
        serde_json::to_string(&split_ahap.continuous).map_err(napi_error_from_error)?;
    let transients = match split_ahap.transients {
        Some(ahap) => Some(serde_json::to_string(&ahap).map_err(napi_error_from_error)?),
        None => None,
    };
    let split_ahap = SplitAhap {
        continuous,
        transients,
    };

    let split_ahap = serde_json::to_value(&split_ahap)?;
    Ok(split_ahap)
}

/// Checks that the supplied JSON string is a valid haptic JSON string of
/// any version.
///
/// If the JSON string is not valid, an Error is thrown.
#[napi]
pub fn validate_json_string(json: String) -> Result<()> {
    from_json(&json, ValidationMode::LegacyCompatibility).map_err(napi_error_from_error)?;
    Ok(())
}
