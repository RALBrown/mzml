use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Debug, Display};

pub trait MassScan {
    ///Return retention time in minutes.
    fn rt(&self) -> Option<uom::si::f32::Time>;
    fn ms_level(&self) -> Option<u16>;
    fn find_cv(&self, name: String) -> Option<&ControlledVocabularyParameter>;
    fn cvs(&self) -> &Vec<ControlledVocabularyParameter>;
    fn ion_fill_time(&self) -> Option<uom::si::f32::Time>;
}
pub trait MassSpectrum {
    type Error: Error + Debug + Display + Send + Sync + 'static;

    fn peaks(&self) -> Result<Cow<[(f64, f64)]>, Self::Error>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "cvParam")]
pub struct ControlledVocabularyParameter {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@unitAccession")]
    pub unit_accession: Option<String>,
    #[serde(rename = "@unitName")]
    pub unit_name: Option<String>,
}
