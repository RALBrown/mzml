use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use zune_inflate::DeflateDecoder;

use crate::mass_spectrum::ControlledVocabularyParameter;
use crate::units::{MsUnit, TimeUnit};

use super::MzMLParseError;

pub(crate) fn base64_decode(data: String) -> Result<Vec<u8>, MzMLParseError> {
    Ok(general_purpose::STANDARD.decode(data)?)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BinaryDataArrayList {
    #[serde(rename = "@count")]
    count: u16,
    #[serde(rename = "binaryDataArray")]
    pub(crate) arrays: Vec<BinaryDataArray>,
}

impl BinaryDataArrayList {
    pub(crate) fn find_binary_by_cv_name(&self, cv_name: &str) -> Option<&BinaryDataArray> {
        self.arrays.iter().find(|array| {
            array.cv_param.iter().any(|c| c.name.contains(cv_name))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BinaryDataArray {
    #[serde(rename = "@encodedLength")]
    encoded_length: usize,
    pub(crate) cv_param: Vec<ControlledVocabularyParameter>,
    binary: String,
}

impl BinaryDataArray {
    fn find_zlib_and_float_size(&self) -> (bool, u8) {
        let mut zlib = false;
        let mut float_size: u8 = 64;
        for param in self.cv_param.iter() {
            let name = &param.name;
            if name.contains("32-bit float") {
                float_size = 32;
            }
            if name.contains("64-bit float") {
                float_size = 64;
            }
            if name.contains("zlib") {
                zlib = true;
            }
        }
        (zlib, float_size)
    }

    pub(crate) fn ms_unit(&self) -> Option<MsUnit> {
        MsUnit::from_cv_params(&self.cv_param)
    }

    /// Decode as a time array, applying unit conversion to SI base units (seconds).
    pub(crate) fn decode_as_time(&self) -> Result<Vec<uom::si::f64::Time>, MzMLParseError> {
        let raw = self.decode()?;
        let unit = TimeUnit::try_from_cv_params(&self.cv_param).unwrap_or(TimeUnit::Second);
        Ok(raw.into_iter().map(|v| unit.to_quantity_f64(v)).collect())
    }

    pub(crate) fn decode(&self) -> Result<Vec<f64>, MzMLParseError> {
        let mut binary = base64_decode(self.binary.clone())?;
        let (zlib, float_size) = self.find_zlib_and_float_size();
        if zlib {
            let mut decoder = DeflateDecoder::new(&binary);
            binary = decoder.decode_zlib()?;
        }
        let mut data = Vec::new();
        match float_size {
            64 => {
                for chunk in binary.chunks(8) {
                    if chunk.len() == 8 {
                        let mut buffer = [0u8; 8];
                        buffer.copy_from_slice(chunk);
                        data.push(f64::from_le_bytes(buffer));
                    }
                }
            }
            32 => {
                for chunk in binary.chunks(4) {
                    if chunk.len() == 4 {
                        let mut buffer = [0u8; 4];
                        buffer.copy_from_slice(chunk);
                        data.push(f32::from_le_bytes(buffer) as f64);
                    }
                }
            }
            _ => panic!("Unknown data size: f_{} for binary array", float_size),
        }
        Ok(data)
    }
}
