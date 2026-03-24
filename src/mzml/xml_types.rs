use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

use crate::mass_spectrum::MassScan;

use super::chromatogram::ChromatogramList;
use super::spectrum::ScanWithoutData;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "indexedmzML")]
pub(super) struct IndexedMzML {
    #[serde(rename = "mzML")]
    pub(super) mzml: MzML<ScanWithoutData>,
    pub(super) index_list: Option<IndexList>,
    pub(super) index: Option<Index>,
    index_list_offset: usize,
    file_checksum: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "mzML")]
pub(super) struct MzML<T: MassScan> {
    software_list: SoftwareList,
    pub(super) run: MzMLRun<T>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct IndexList {
    #[serde(rename = "@count")]
    pub(super) count: u32,
    #[serde(rename = "$value")]
    pub(super) indexs: Vec<Index>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct Index {
    #[serde(rename = "@name")]
    pub(super) name: String,
    #[serde(rename = "$value")]
    pub(super) offsets: Vec<Offset>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Offset {
    #[serde(rename = "@idRef")]
    pub(super) id_ref: String,
    #[serde(rename = "$value")]
    pub(super) offset: usize,
}

impl Hash for Offset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id_ref.hash(state);
    }
}

impl PartialEq for Offset {
    fn eq(&self, other: &Self) -> bool {
        self.id_ref == other.id_ref
    }
}

impl Eq for Offset {}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SoftwareList {
    #[serde(rename = "$value")]
    software_list: Vec<Software>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Software {
    #[serde(rename = "@id")]
    name: String,
    #[serde(rename = "@version")]
    version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "run")]
pub(super) struct MzMLRun<T: MassScan> {
    pub(super) spectrum_list: SpectrumList<T>,
    pub(super) chromatogram_list: ChromatogramList,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct SpectrumList<T: MassScan> {
    #[serde(rename = "@count")]
    count: usize,
    #[serde(rename = "$value")]
    pub(super) spectra: Vec<T>,
}
