use quick_xml::de::{from_reader, from_str};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use thiserror::Error;

pub mod binary;
pub mod chromatogram;
pub mod spectrum;
pub mod xml_types;

pub use chromatogram::Chromatogram;
pub use spectrum::{IsolationWindow, Precursor, PrecursorList, Scan, ScanWithData, ScanWithoutData};

use crate::mass_spectrum::{MassScan, MassSpectrum};
use spectrum::ScanData;
use xml_types::{IndexedMzML, IndexList};

#[derive(Error, Debug)]
pub enum MzMLParseError {
    #[error("MzML parsing error: {0}")]
    MzMLFormatError(#[from] quick_xml::de::DeError),
    #[error("zlib decoding error: {0}")]
    ZlibDecodeError(#[from] zune_inflate::errors::InflateDecodeErrors),
    #[error("Base64 parsing error, scan data is not parsable: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
}

/// Lazily-loaded indexed mzML file. Metadata is loaded upfront; binary
/// spectrum data is fetched from disk on demand.
#[derive(Debug)]
pub struct LazyMzML {
    mzml_struct: IndexedMzML,
    file: File,
    scan_offsets: HashMap<String, usize>,
    chromatogram_offsets: HashMap<String, usize>,
}

impl LazyMzML {
    /// Open an indexed mzML file. The index is parsed immediately; spectrum
    /// binary data is not read until [`iter_spectrum`] or [`fetch_scan_data`] is called.
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
                    temp_index_list = IndexList { count: 1, indexs: vec };
                    &temp_index_list
                } else {
                    temp_index_list = IndexList { count: 0, indexs: Vec::new() };
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
            scan_offsets,
            chromatogram_offsets,
        })
    }

    /// Iterate over scan metadata without loading binary data.
    pub fn iter_scan(&self) -> impl Iterator<Item = &ScanWithoutData> {
        self.mzml_struct.mzml.run.spectrum_list.spectra.iter()
    }

    /// Iterate over preloaded chromatograms.
    pub fn iter_chromatogram(&self) -> impl Iterator<Item = &Chromatogram> {
        self.mzml_struct.mzml.run.chromatogram_list.chromatograms.iter()
    }

    /// Iterate over spectra, loading binary data for each from disk.
    pub fn iter_spectrum(&self) -> impl Iterator<Item = impl MassScan + MassSpectrum> + '_ {
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

    /// Fetch binary data for a single scan from disk.
    pub fn fetch_scan_data(&self, scan: &ScanWithoutData) -> Option<ScanWithData> {
        const BUFFER_SIZE: usize = 8000;
        let offset = self.scan_offsets.get(&scan.id)?;
        let tag = b"</spectrum>";
        let mut xml_bytes = Vec::new();
        let mut buffer = [0u8; BUFFER_SIZE];
        let mut reader = BufReader::new(&self.file);
        reader.seek(SeekFrom::Start(*offset as u64)).unwrap();
        loop {
            let n = reader.read(&mut buffer).ok()?;
            if n == 0 {
                break;
            }
            xml_bytes.extend_from_slice(&buffer[..n]);
            let search_start = xml_bytes.len().saturating_sub(n + tag.len());
            if let Some(pos) = xml_bytes[search_start..]
                .windows(tag.len())
                .position(|window| window == tag)
            {
                xml_bytes.truncate(search_start + pos + tag.len());
                break;
            }
        }
        let final_string = String::from_utf8(xml_bytes).ok()?;
        let spectrum: ScanData = from_str(&final_string).unwrap();
        Some(ScanWithData {
            scan: scan.to_owned(),
            binary_data_array_list: spectrum.binary_data_array_list,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::mzml::LazyMzML;

    #[test]
    fn integration() {
        use rayon::iter::ParallelBridge;
        use rayon::prelude::ParallelIterator;

        let file =
            std::fs::File::open(std::path::Path::new(r"test_data\small.pwiz.1.1.mzML")).unwrap();
        let mzml_struct = LazyMzML::new(file).unwrap();
        let intensities: Vec<_> = mzml_struct
            .iter_spectrum()
            .par_bridge()
            .map(|spectrum| {
                use crate::mass_spectrum::{MassScan, MassSpectrum};
                let time = spectrum.rt().unwrap();
                let array = spectrum.peaks();
                match array {
                    Ok(intensity) => (intensity[0].0, time),
                    _ => panic!(),
                }
            })
            .collect();
        let total: f64 = intensities.iter().map(|a| a.0).sum();
        let total_time: uom::si::f32::Time = intensities.iter().map(|a| a.1).sum();
        println!(
            "{}",
            total_time.into_format_args(uom::si::time::minute, uom::fmt::DisplayStyle::Abbreviation)
        );
        println!("{}", total);
        assert_eq!(total, 9938.47898941423);

        for s in mzml_struct.iter_scan() {
            println!("{:?}", s.precursor_list);
        }

        let chrom = mzml_struct
            .iter_chromatogram()
            .find(|c| c.id == "TIC")
            .unwrap();
        #[allow(deprecated)]
        for point in chrom.trace().unwrap() {
            print!("{point:?}");
        }
    }
}
