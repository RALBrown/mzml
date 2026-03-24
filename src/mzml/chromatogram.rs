use serde::{Deserialize, Serialize};

use super::binary::BinaryDataArrayList;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChromatogramList {
    #[serde(rename = "@count")]
    count: u16,
    #[serde(rename = "$value")]
    pub(crate) chromatograms: Vec<Chromatogram>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Chromatogram {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@index")]
    index: u16,
    binary_data_array_list: BinaryDataArrayList,
}

impl Chromatogram {
    #[deprecated(note = "Use `trace_with_time` instead to get unit-typed retention times.")]
    pub fn trace(&self) -> Option<Vec<(f64, f64)>> {
        let retention_times_blob = self.binary_data_array_list.find_binary_by_cv_name("time array")?;
        let intensities_blob = self.binary_data_array_list.find_binary_by_cv_name("intensity array")?;
        Some(
            retention_times_blob.decode().ok()?.into_iter()
                .zip(intensities_blob.decode().ok()?)
                .collect(),
        )
    }

    pub fn trace_with_time(&self) -> Option<Vec<(uom::si::f64::Time, f64)>> {
        let retention_times_blob = self.binary_data_array_list.find_binary_by_cv_name("time array")?;
        let intensities_blob = self.binary_data_array_list.find_binary_by_cv_name("intensity array")?;
        Some(
            retention_times_blob.decode_as_time().ok()?.into_iter()
                .zip(intensities_blob.decode().ok()?)
                .collect(),
        )
    }
}
