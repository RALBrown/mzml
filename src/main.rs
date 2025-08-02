#![allow(dead_code)]
#![allow(unused_variables)]
use base64::{engine::general_purpose, Engine as _};
use mzml::LazyMzML;
use quick_xml::de::{from_reader, from_str};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Read, Seek, SeekFrom};
use thiserror::Error;
use zune_inflate::DeflateDecoder;
use mzml::mass_spectrum::MassScan;

fn main() {
    let file =
        File::open(r"C:\Users\Robert\Documents\GitHub\mzml\test_data\small.pwiz.1.1.mzML").unwrap();
    let mzml_struct = LazyMzML::new(file).unwrap();
    for spectrum in mzml_struct.iter_scan()
    {
        println!("{:?}", spectrum.rt());
    }
}
