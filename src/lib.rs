#![allow(dead_code)]
#![allow(unused_variables)]
use base64::{engine::general_purpose, Engine as _};
use quick_xml::de::{from_reader, from_str};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Read, Seek, SeekFrom};
use thiserror::Error;
use uom::si::f32::Time;
use uom::si::time::{minute, second};
use zune_inflate::DeflateDecoder;

pub mod mass_spectrum;
use mass_spectrum::{ControlledVocabularyParameter, MassScan, MassSpectrum};

fn base64_decode(data: String) -> Result<Vec<u8>, MzMLParseError> {
    Ok(general_purpose::STANDARD.decode(data)?)
}
/**A structure holding the scan information of an Inbdexed mzml file.
Spectrum data will be loaded lazily from disk when objects bearing the MassSpectrum trait are retreived.
*/
#[derive(Debug)]
pub struct LazyMzML {
    mzml_struct: IndexedMzML,
    file: File,
    scan_offsets: HashMap<String, usize>,
    chromatogram_offsets: HashMap<String, usize>,
}
impl LazyMzML {
    ///Create a new LazyMzML from an indexed mzml file.
    pub fn new(mzml_file: File) -> Result<Self, MzMLParseError> {
        let buffreader = BufReader::new(&mzml_file);
        let mzml: IndexedMzML = from_reader(buffreader)?;
        let mut scan_offsets: HashMap<String, usize> = HashMap::new();
        let temp_index_list: IndexList;
        let index_list = match &mzml.index_list {
            Some(i) => i,
            None => {
                if let Some(index) = &mzml.index {
                    let mut vec = Vec::new();
                    vec.push(index.to_owned());
                    temp_index_list = IndexList {
                        count: 1,
                        indexs: vec,
                    };
                    &temp_index_list
                } else {
                    temp_index_list = IndexList {
                        count: 0,
                        indexs: Vec::new(),
                    };
                    &temp_index_list
                }
            }
        };
        index_list
            .indexs
            .iter()
            .find(|index| index.name == "spectrum")
            .expect("All indexed mzML should have a spectrum index")
            .offsets
            .iter()
            .for_each(|offset| {
                scan_offsets.insert(offset.id_ref.clone(), offset.offset);
            });
        let mut chromatogram_offsets: HashMap<String, usize> = HashMap::new();
        index_list
            .indexs
            .iter()
            .find(|index| index.name == "chromatogram")
            .expect("All indexed mzML should have a chromatogram index")
            .offsets
            .iter()
            .for_each(|offset| {
                chromatogram_offsets.insert(offset.id_ref.clone(), offset.offset);
            });
        Ok(LazyMzML {
            mzml_struct: mzml,
            file: mzml_file,
            scan_offsets: scan_offsets,
            chromatogram_offsets: chromatogram_offsets,
        })
    }
}

impl<'a> LazyMzML {
    /**Return an iterator of MassScan objects contained in the LazyMzML.
     */
    pub fn iter_scan(&'a self) -> impl Iterator<Item = &ScanWithoutData> + 'a {
        self.mzml_struct.mzml.run.spectrum_list.spectra.iter()
    }

    /**Return an iterator of MassScan objects the underlying data is additionally loaded from disk to create MassSpectrum.
     */
    pub fn iter_spectrum(&'a self) -> impl Iterator<Item = impl MassScan + MassSpectrum> + 'a {
        self.mzml_struct
            .mzml
            .run
            .spectrum_list
            .spectra
            .iter()
            .map(|s| {
                self.fetch_scan_data(s)
                    .expect("Spectrum data should be retrievable")
            })
    }

    pub fn fetch_scan_data(&self, scan: &ScanWithoutData) -> Option<ScanWithData> {
        const BUFFER_SIZE: usize = 8000;
        let offset = self.scan_offsets.get(&(scan.id))?;
        let file = &self.file;
        let mut xml_string = String::from("");
        let mut buffer = [0; BUFFER_SIZE];
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(*offset as u64)).unwrap();
        let mut number_of_buffers: usize = 0;
        loop {
            let number_bytes = reader.read(&mut buffer[..]).ok()?;
            xml_string.push_str(std::str::from_utf8(&buffer[..number_bytes]).ok()?);
            if let Some(n) = xml_string[xml_string
                .len()
                .checked_sub(BUFFER_SIZE)
                .unwrap_or_default()..]
                .find(r"</spectrum>")
            {
                xml_string.truncate(number_of_buffers * BUFFER_SIZE + n + 11);
                break;
            }
            number_of_buffers += 1;
        }
        let spectrum: ScanWithData = from_str(&xml_string).unwrap();
        Some(spectrum)
    }
}

#[derive(Error, Debug)]
pub enum MzMLParseError {
    #[error("MzML parsing error: {0}")]
    MzMLFormatError(#[from] quick_xml::de::DeError),
    #[error("zlib decoding error: {0}")]
    ZlibDecodeError(#[from] zune_inflate::errors::InflateDecodeErrors),
    #[error("Base64 parsing error, scan data is not parsable: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "indexedmzML")]
struct IndexedMzML {
    #[serde(rename = "mzML")]
    mzml: MzML<ScanWithoutData>,
    index_list: Option<IndexList>,
    index: Option<Index>,
    index_list_offset: usize,
    file_checksum: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "mzML")]
struct MzML<T: MassScan> {
    software_list: SoftwareList,
    run: MzMLRun<T>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct IndexList {
    #[serde(rename = "@count")]
    count: u32,
    #[serde(rename = "$value")]
    indexs: Vec<Index>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Index {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "$value")]
    offsets: Vec<Offset>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Offset {
    #[serde(rename = "@idRef")]
    id_ref: String,
    #[serde(rename = "$value")]
    offset: usize,
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
struct MzMLRun<T: MassScan> {
    spectrum_list: SpectrumList<T>,
    chromatogram_list: ChromatogramList,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SpectrumList<T: MassScan> {
    #[serde(rename = "@count")]
    count: usize,
    #[serde(rename = "$value")]
    spectra: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ChromatogramList {
    #[serde(rename = "@count")]
    count: u16,
    #[serde(rename = "$value")]
    chromatograms: Vec<Chromatogram>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Chromatogram {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@index")]
    index: u16,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "spectrum")]
#[serde(rename_all = "camelCase")]
pub struct ScanWithData {
    #[serde(rename = "@index")]
    index: usize,
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@defaultArrayLength")]
    default_array_length: usize,
    cv_param: Vec<ControlledVocabularyParameter>,
    #[serde(default)]
    pub precursor_list: Option<PrecursorList>,
    scan_list: ScanList,
    binary_data_array_list: BinaryDataArrayList,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename = "spectrum")]
#[serde(rename_all = "camelCase")]
pub struct ScanWithoutData {
    #[serde(rename = "@index")]
    index: usize,
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@defaultArrayLength")]
    default_array_length: usize,
    cv_param: Vec<ControlledVocabularyParameter>,
    #[serde(default)]
    pub precursor_list: Option<PrecursorList>,
    scan_list: ScanList,
}

impl MassSpectrum for ScanWithData {
    type Err = MzMLParseError;
    fn peaks(&self) -> Result<Vec<(f64, f64)>, Self::Err> {
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
        Ok(mz.into_iter().zip(intensity.into_iter()).collect())
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
            .find(|c| c.name.find("scan start time").is_some())?;
        let time: f32 = rt_cv.value.parse().unwrap();
        let unit_string = rt_cv.unit_name.as_ref()?;
        match &unit_string[..] {
            "minute" => Some(Time::new::<minute>(time)),
            "second" => Some(Time::new::<second>(time)),
            _ => Some(Time::new::<minute>(time)),
        }
    }
    fn ms_level(&self) -> Option<u16> {
        self.cv_param
            .iter()
            .find(|c| c.name.find("ms level").is_some())?
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
}
impl MassScan for ScanWithData {
    fn rt(&self) -> Option<uom::si::f32::Time> {
        let rt_cv = self
            .scan_list
            .scan
            .first()?
            .cv_param
            .iter()
            .find(|c| c.name.find("scan start time").is_some())?;
        let time: f32 = rt_cv.value.parse().unwrap();
        let unit_string = rt_cv.unit_name.as_ref()?;
        match &unit_string[..] {
            "minute" => Some(Time::new::<minute>(time)),
            "second" => Some(Time::new::<second>(time)),
            _ => Some(Time::new::<minute>(time)),
        }
    }
    fn ms_level(&self) -> Option<u16> {
        self.cv_param
            .iter()
            .find(|c| c.name.find("ms level").is_some())?
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
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ScanList {
    scan: Vec<Scan>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Scan {
    cv_param: Vec<ControlledVocabularyParameter>,
}
impl Scan {
    pub fn find_cv(&self, name: String) -> Option<&ControlledVocabularyParameter> {
        self.cv_param.iter().find(|cv| cv.name == name)
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct BinaryDataArrayList {
    #[serde(rename = "@count")]
    count: u16,
    #[serde(rename = "binaryDataArray")]
    arrays: Vec<BinaryDataArray>,
}
impl BinaryDataArrayList {
    ///Return the first BinaryDataArray that contains a CV element with the input name
    pub fn find_binary_by_cv_name(&self, cv_name: &str) -> Option<&BinaryDataArray> {
        self.arrays.iter().find(|array| {
            array
                .cv_param
                .iter()
                .any(|c| c.name.find(cv_name).is_some())
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct BinaryDataArray {
    #[serde(rename = "@encodedLength")]
    encoded_length: usize,
    cv_param: Vec<ControlledVocabularyParameter>,
    binary: String,
}
impl BinaryDataArray {
    fn find_zlib_and_float_size(&self) -> (bool, u8) {
        let mut zlib = false;
        let mut float_size: u8 = 64;
        for param in self.cv_param.iter() {
            let name = &param.name;
            if let Some(n) = name.find("32-bit float") {
                float_size = 32;
            }
            if let Some(n) = name.find("64-bit float") {
                float_size = 64;
            }
            if let Some(n) = name.find("zlib") {
                zlib = true;
            }
        }
        (zlib, float_size)
    }
    /**Return the decoded data as a Vec.
     */
    fn decode(&self) -> Result<Vec<f64>, MzMLParseError> {
        let mut binary = base64_decode(self.binary.clone())?;
        let (zlib, float_size) = self.find_zlib_and_float_size();
        if zlib {
            let mut decoder = DeflateDecoder::new(&binary);
            binary = decoder.decode_zlib()?;
        }
        let mut data = Vec::new();
        match float_size {
            64 => {
                let chunks = binary.chunks(8);
                for chunk in chunks {
                    let mut buffer: [u8; 8] = [0 as u8; 8];
                    if chunk.len() == 8 && buffer.len() == 8 {
                        for (i, target) in buffer.iter_mut().enumerate() {
                            *target = chunk[i];
                        }

                        data.push(f64::from_le_bytes(buffer));
                    }
                }
            }
            32 => {
                let chunks = binary.chunks(4);
                for chunk in chunks {
                    let mut buffer: [u8; 4] = [0 as u8; 4];
                    if chunk.len() == 4 && buffer.len() == 4 {
                        for (i, target) in buffer.iter_mut().enumerate() {
                            *target = chunk[i];
                        }

                        data.push(f32::from_le_bytes(buffer) as f64);
                    }
                }
            }
            _ => panic!("Unknow data size: f_{} for binary array", float_size),
        };
        Ok(data)
    }
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrecursorList{
    #[serde(rename = "$value")]
    precursors: Vec<Precursor>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Precursor {
    #[serde(rename = "@spectrumRef")]
    reference_spectrum: String,
    #[serde(default)]
    pub isolation_window: IsolationWindow,

}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IsolationWindow {
    pub cv_param: Vec<ControlledVocabularyParameter>,
}
impl Default for IsolationWindow {
    fn default() -> Self {
        IsolationWindow {
            cv_param: Vec::new(),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn integration() {
        use rayon::iter::ParallelBridge;
        use rayon::prelude::ParallelIterator;
        
        // let resp = reqwest::blocking::get("https://github.com/HUPO-PSI/mzML/blob/master/examples/2min.mzML").expect("request failed");
        // let body = resp.text().expect("body invalid");
        // let mut file = tempfile::tempfile().unwrap();
        // std::io::copy(&mut body.as_bytes(), &mut file).expect("failed to copy content");
        let file = std::fs::File::open(std::path::Path::new(r"test_data\small.pwiz.1.1.mzML")).unwrap();
        let mzml_struct = LazyMzML::new(file).unwrap();
        let intensities: Vec<_> = mzml_struct
            .iter_spectrum()
            .par_bridge()
            .map(|spectrum| {
                let time = spectrum.rt().unwrap();
                let array = spectrum.peaks();
                match array {
                    Ok(intensity) => return (intensity[0].0, time),
                    _ => panic!(),
                }
            })
            .collect();
        let total: f64 = intensities.iter().map(|a| a.0).sum();
        let total_time: Time = intensities.iter().map(|a| a.1).sum();
        println!(
            "{}",
            total_time
                .into_format_args(uom::si::time::minute, uom::fmt::DisplayStyle::Abbreviation)
        );
        println!("{}", total);
        assert_eq!(total, 9938.47898941423);
        for s in mzml_struct.iter_scan() {
            println!("{:?}", s.precursor_list);
        }
    }
}
