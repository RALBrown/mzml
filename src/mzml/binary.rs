use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use zune_inflate::DeflateDecoder;

use crate::mass_spectrum::ControlledVocabularyParameter;
use crate::units::TimeUnit;

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
        self.arrays
            .iter()
            .find(|array| array.cv_param.iter().any(|c| c.name.contains(cv_name)))
    }

    pub(crate) fn find_binary_by_cv_accession(&self, accession: &str) -> Option<&BinaryDataArray> {
        self.arrays
            .iter()
            .find(|array| array.cv_param.iter().any(|c| c.accession == accession))
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

enum Encoding {
    Float { zlib: bool, bits: u8 },
    NumpressLinear { zlib: bool },
    NumpressPic { zlib: bool },
    NumpressSlof { zlib: bool },
}

impl BinaryDataArray {
    fn find_encoding(&self) -> Encoding {
        let mut zlib = false;
        let mut float_size: u8 = 64;
        let mut numpress: Option<Encoding> = None;
        for param in self.cv_param.iter() {
            match param.accession.as_str() {
                "MS:1000521" => float_size = 32,                                    // 32-bit float
                "MS:1000523" => float_size = 64,                                    // 64-bit float
                "MS:1000574" => zlib = true,                                        // zlib compression
                "MS:1002312" => numpress = Some(Encoding::NumpressLinear { zlib: false }),
                "MS:1002313" => numpress = Some(Encoding::NumpressPic { zlib: false }),
                "MS:1002314" => numpress = Some(Encoding::NumpressSlof { zlib: false }),
                _ => {}
            }
        }
        match numpress {
            Some(Encoding::NumpressLinear { .. }) => Encoding::NumpressLinear { zlib },
            Some(Encoding::NumpressPic { .. })    => Encoding::NumpressPic { zlib },
            Some(Encoding::NumpressSlof { .. })   => Encoding::NumpressSlof { zlib },
            _ => Encoding::Float { zlib, bits: float_size },
        }
    }

    /// Decode as a time array, applying unit conversion to SI base units (seconds).
    pub(crate) fn decode_as_time(&self) -> Result<Vec<uom::si::f64::Time>, MzMLParseError> {
        let raw = self.decode()?;
        let unit = TimeUnit::try_from_cv_params(&self.cv_param).unwrap_or(TimeUnit::Second);
        Ok(raw.into_iter().map(|v| unit.to_quantity_f64(v)).collect())
    }

    pub(crate) fn decode(&self) -> Result<Vec<f64>, MzMLParseError> {
        let binary = base64_decode(self.binary.clone())?;
        match self.find_encoding() {
            Encoding::Float { zlib, bits } => {
                let binary = if zlib {
                    let mut decoder = DeflateDecoder::new(&binary);
                    decoder.decode_zlib()?
                } else {
                    binary
                };
                let mut data = Vec::new();
                match bits {
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
                    _ => panic!("Unknown data size: f_{bits} for binary array"),
                }
                Ok(data)
            }
            Encoding::NumpressLinear { zlib } | Encoding::NumpressPic { zlib } | Encoding::NumpressSlof { zlib } => {
                let binary = if zlib {
                    let mut decoder = DeflateDecoder::new(&binary);
                    decoder.decode_zlib()?
                } else {
                    binary
                };
                let mut data = Vec::new();
                let decode_fn = match self.find_encoding() {
                    Encoding::NumpressLinear { .. } => numpress_rs::decode_linear,
                    Encoding::NumpressPic { .. }    => numpress_rs::decode_pic,
                    Encoding::NumpressSlof { .. }   => numpress_rs::decode_slof,
                    _ => unreachable!(),
                };
                decode_fn(&binary, &mut data)
                    .map_err(|e| MzMLParseError::NumpressError(*e.kind()))?;
                Ok(data)
            }
        }
    }
}
