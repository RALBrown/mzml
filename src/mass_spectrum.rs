use serde::{Deserialize, Serialize};

pub trait MassScan {
    ///Return retention time in minutes.
    fn rt(&self) -> Option<uom::si::f32::Time>;
    fn ms_level(&self) -> Option<u16>;
    fn find_cv(&self, name: String) -> Option<&ControlledVocabularyParameter>;
    fn cvs(&self) -> &Vec<ControlledVocabularyParameter>;
}
pub trait MassSpectrum {
    fn peaks(&self) -> Result<Vec<(f64, f64)>, crate::MzMLParseError>;
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "cvParam")]
pub struct ControlledVocabularyParameter {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@unitName")]
    pub unit_name: Option<String>,
}
