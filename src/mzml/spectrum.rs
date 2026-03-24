use delegate::delegate;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::ops::Deref;

use crate::mass_spectrum::{ControlledVocabularyParameter, MassScan, MassSpectrum};
use crate::units::{cv_to_time_f32, TimeUnit};

use super::binary::BinaryDataArrayList;
use super::MzMLParseError;

/// Intermediate deserialization target when fetching binary data from disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename = "spectrum")]
#[serde(rename_all = "camelCase")]
pub(super) struct ScanData {
    pub(super) binary_data_array_list: BinaryDataArrayList,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename = "spectrum")]
#[serde(rename_all = "camelCase")]
pub struct ScanWithData {
    #[serde(flatten)]
    pub(super) scan: ScanWithoutData,
    pub(super) binary_data_array_list: BinaryDataArrayList,
}

impl ScanWithData {
    delegate! {
        to self.scan {
            pub fn cvs(&self) -> &Vec<ControlledVocabularyParameter>;
            pub fn find_cv(&self, name: String) -> Option<&ControlledVocabularyParameter>;
            pub fn rt(&self) -> Option<uom::si::f32::Time>;
            pub fn ms_level(&self) -> Option<u16>;
            pub fn ion_fill_time(&self) -> Option<uom::si::f32::Time>;
        }
    }
}

impl Deref for ScanWithData {
    type Target = ScanWithoutData;

    fn deref(&self) -> &Self::Target {
        &self.scan
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename = "spectrum")]
#[serde(rename_all = "camelCase")]
pub struct ScanWithoutData {
    #[serde(rename = "@index")]
    pub(super) index: usize,
    #[serde(rename = "@id")]
    pub(super) id: String,
    #[serde(rename = "@defaultArrayLength")]
    pub(super) default_array_length: usize,
    pub(super) cv_param: Vec<ControlledVocabularyParameter>,
    #[serde(default)]
    pub precursor_list: Option<PrecursorList>,
    pub(super) scan_list: ScanList,
}

impl MassSpectrum for ScanWithData {
    type Error = MzMLParseError;

    fn peaks(&self) -> Result<Cow<[(f64, f64)]>, MzMLParseError> {
        let mz_array = self
            .binary_data_array_list
            .find_binary_by_cv_name("m/z array")
            .expect("All spectra should have an m/z array");
        let intensity_array = self
            .binary_data_array_list
            .find_binary_by_cv_name("intensity array")
            .expect("All spectra should have an intensity array");
        let mz = mz_array.decode()?;
        let intensity = intensity_array.decode()?;
        Ok(Cow::Owned(mz.into_iter().zip(intensity.into_iter()).collect()))
    }
}

impl MassScan for ScanWithoutData {
    fn rt(&self) -> Option<uom::si::f32::Time> {
        let rt_cv = self
            .scan_list
            .scan
            .first()?
            .cv_param
            .iter()
            .find(|c| c.name.contains("scan start time"))?;
        cv_to_time_f32(rt_cv, TimeUnit::Minute)
    }

    fn ms_level(&self) -> Option<u16> {
        self.cv_param
            .iter()
            .find(|c| c.name.contains("ms level"))?
            .value
            .parse()
            .ok()
    }

    fn cvs(&self) -> &Vec<ControlledVocabularyParameter> {
        &self.cv_param
    }

    fn find_cv(&self, name: String) -> Option<&ControlledVocabularyParameter> {
        self.cv_param.iter().find(|cv| cv.name == name)
    }

    fn ion_fill_time(&self) -> Option<uom::si::f32::Time> {
        let rt_cv = self
            .scan_list
            .scan
            .first()?
            .cv_param
            .iter()
            .find(|c| c.name.contains("ion injection time"))?;
        cv_to_time_f32(rt_cv, TimeUnit::Millisecond)
    }
}

impl MassScan for ScanWithData {
    fn rt(&self) -> Option<uom::si::f32::Time> {
        self.rt()
    }

    fn ms_level(&self) -> Option<u16> {
        self.ms_level()
    }

    fn cvs(&self) -> &Vec<ControlledVocabularyParameter> {
        &self.cv_param
    }

    fn find_cv(&self, name: String) -> Option<&ControlledVocabularyParameter> {
        self.cv_param.iter().find(|cv| cv.name == name)
    }

    fn ion_fill_time(&self) -> Option<uom::si::f32::Time> {
        self.ion_fill_time()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct ScanList {
    pub(super) scan: Vec<Scan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub cv_param: Vec<ControlledVocabularyParameter>,
}

impl Scan {
    pub fn find_cv(&self, name: String) -> Option<&ControlledVocabularyParameter> {
        self.cv_param.iter().find(|cv| cv.name == name)
    }

    pub fn ion_fill_time(&self) -> Option<uom::si::f32::Time> {
        let rt_cv = self
            .cv_param
            .iter()
            .find(|c| c.name.contains("ion injection time"))?;
        cv_to_time_f32(rt_cv, TimeUnit::Millisecond)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrecursorList {
    #[serde(rename = "$value")]
    pub precursors: Vec<Precursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Precursor {
    #[serde(rename = "@spectrumRef")]
    pub reference_spectrum: Option<String>,
    #[serde(default)]
    pub isolation_window: IsolationWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IsolationWindow {
    pub cv_param: Vec<ControlledVocabularyParameter>,
}

impl Default for IsolationWindow {
    fn default() -> Self {
        IsolationWindow { cv_param: Vec::new() }
    }
}
