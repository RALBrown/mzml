use thiserror::Error;
use uom::si::energy::electronvolt;
use uom::si::f64::{Energy, Ratio, Time};
use uom::si::ratio::percent;
use uom::si::time::{hour, microsecond, millisecond, minute, second};

use crate::mass_spectrum::ControlledVocabularyParameter;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnitConversionError {
    #[error("unrecognized unit accession: {0}")]
    UnrecognizedAccession(String),
    #[error("unrecognized unit name: {0}")]
    UnrecognizedName(String),
    #[error("no unit information present")]
    NoUnitPresent,
}

// ── TimeUnit ──────────────────────────────────────────────────────────────────

/// Time units from the Units Ontology (UO) namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Microsecond, // UO:0000029
    Millisecond, // UO:0000028
    Second,      // UO:0000010
    Minute,      // UO:0000031
    Hour,        // UO:0000032
}

impl TimeUnit {
    pub fn from_accession(acc: &str) -> Option<Self> {
        match acc {
            "UO:0000029" => Some(Self::Microsecond),
            "UO:0000028" => Some(Self::Millisecond),
            "UO:0000010" => Some(Self::Second),
            "UO:0000031" => Some(Self::Minute),
            "UO:0000032" => Some(Self::Hour),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "microsecond" => Some(Self::Microsecond),
            "millisecond" => Some(Self::Millisecond),
            "second" => Some(Self::Second),
            "minute" => Some(Self::Minute),
            "hour" => Some(Self::Hour),
            _ => None,
        }
    }

    pub fn to_quantity_f64(self, value: f64) -> Time {
        match self {
            Self::Microsecond => Time::new::<microsecond>(value),
            Self::Millisecond => Time::new::<millisecond>(value),
            Self::Second => Time::new::<second>(value),
            Self::Minute => Time::new::<minute>(value),
            Self::Hour => Time::new::<hour>(value),
        }
    }

    pub fn to_quantity_f32(self, value: f32) -> uom::si::f32::Time {
        use uom::si::f32::Time as TimeF32;
        match self {
            Self::Microsecond => TimeF32::new::<microsecond>(value),
            Self::Millisecond => TimeF32::new::<millisecond>(value),
            Self::Second => TimeF32::new::<second>(value),
            Self::Minute => TimeF32::new::<minute>(value),
            Self::Hour => TimeF32::new::<hour>(value),
        }
    }

    /// Find the first CVParam in a slice that resolves to a TimeUnit.
    pub fn try_from_cv_params(params: &[ControlledVocabularyParameter]) -> Option<Self> {
        params.iter().find_map(|cv| Self::try_from(cv).ok())
    }
}

impl TryFrom<&ControlledVocabularyParameter> for TimeUnit {
    type Error = UnitConversionError;

    fn try_from(cv: &ControlledVocabularyParameter) -> Result<Self, Self::Error> {
        if let Some(acc) = &cv.unit_accession {
            if let Some(unit) = Self::from_accession(acc) {
                return Ok(unit);
            }
        }
        if let Some(name) = &cv.unit_name {
            if let Some(unit) = Self::from_name(name) {
                return Ok(unit);
            }
            return Err(UnitConversionError::UnrecognizedName(name.clone()));
        }
        Err(UnitConversionError::NoUnitPresent)
    }
}

// ── IntensityUnit ─────────────────────────────────────────────────────────────

/// Intensity units from the PSI-MS ontology (MS namespace).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntensityUnit {
    NumberOfCounts,    // MS:1000131
    PercentOfBasePeak, // MS:1000132
}

impl IntensityUnit {
    pub fn from_accession(acc: &str) -> Option<Self> {
        match acc {
            "MS:1000131" => Some(Self::NumberOfCounts),
            "MS:1000132" => Some(Self::PercentOfBasePeak),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "number of counts" => Some(Self::NumberOfCounts),
            "percent of base peak" => Some(Self::PercentOfBasePeak),
            _ => None,
        }
    }

    /// Returns a `Ratio` for relative intensity; `None` for raw counts (dimensionless).
    pub fn to_ratio_f64(self, value: f64) -> Option<Ratio> {
        match self {
            Self::PercentOfBasePeak => Some(Ratio::new::<percent>(value)),
            Self::NumberOfCounts => None,
        }
    }
}

impl TryFrom<&ControlledVocabularyParameter> for IntensityUnit {
    type Error = UnitConversionError;

    fn try_from(cv: &ControlledVocabularyParameter) -> Result<Self, Self::Error> {
        if let Some(acc) = &cv.unit_accession {
            if let Some(unit) = Self::from_accession(acc) {
                return Ok(unit);
            }
        }
        if let Some(name) = &cv.unit_name {
            if let Some(unit) = Self::from_name(name) {
                return Ok(unit);
            }
            return Err(UnitConversionError::UnrecognizedName(name.clone()));
        }
        Err(UnitConversionError::NoUnitPresent)
    }
}

// ── EnergyUnit ────────────────────────────────────────────────────────────────

/// Energy units from the Units Ontology (UO) namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyUnit {
    Electronvolt, // UO:0000266
}

impl EnergyUnit {
    pub fn from_accession(acc: &str) -> Option<Self> {
        match acc {
            "UO:0000266" => Some(Self::Electronvolt),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "electronvolt" | "electron volt" => Some(Self::Electronvolt),
            _ => None,
        }
    }

    pub fn to_quantity_f64(self, value: f64) -> Energy {
        match self {
            Self::Electronvolt => Energy::new::<electronvolt>(value),
        }
    }
}

impl TryFrom<&ControlledVocabularyParameter> for EnergyUnit {
    type Error = UnitConversionError;

    fn try_from(cv: &ControlledVocabularyParameter) -> Result<Self, Self::Error> {
        if let Some(acc) = &cv.unit_accession {
            if let Some(unit) = Self::from_accession(acc) {
                return Ok(unit);
            }
        }
        if let Some(name) = &cv.unit_name {
            if let Some(unit) = Self::from_name(name) {
                return Ok(unit);
            }
            return Err(UnitConversionError::UnrecognizedName(name.clone()));
        }
        Err(UnitConversionError::NoUnitPresent)
    }
}

// ── MzValue ───────────────────────────────────────────────────────────────────

/// Mass-to-charge ratio (Thomson). No equivalent SI unit in `uom`.
/// Corresponds to MS:1000040 / unit_name "m/z".
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MzValue(pub f64);

impl MzValue {
    pub fn value(self) -> f64 {
        self.0
    }
}

// ── MsUnit ────────────────────────────────────────────────────────────────────

/// Top-level enum covering all recognised MS/UO unit types.
/// Used to tag both scalar CVParam values and binary data arrays.
#[derive(Debug, Clone, PartialEq)]
pub enum MsUnit {
    Time(TimeUnit),
    Intensity(IntensityUnit),
    Energy(EnergyUnit),
    MassToCharge,    // MS:1000040
    Unknown(String), // accession present but not mapped
}

impl MsUnit {
    /// Resolve from a unit accession string (UO: or MS: namespace).
    pub fn from_accession(acc: &str) -> Self {
        if let Some(u) = TimeUnit::from_accession(acc) {
            return Self::Time(u);
        }
        if let Some(u) = IntensityUnit::from_accession(acc) {
            return Self::Intensity(u);
        }
        if let Some(u) = EnergyUnit::from_accession(acc) {
            return Self::Energy(u);
        }
        if acc == "MS:1000040" {
            return Self::MassToCharge;
        }
        Self::Unknown(acc.to_owned())
    }

    /// Resolve from the first CVParam in a slice that carries unit information.
    pub fn from_cv_params(params: &[ControlledVocabularyParameter]) -> Option<Self> {
        for cv in params {
            if let Some(acc) = &cv.unit_accession {
                return Some(Self::from_accession(acc));
            }
            if let Some(name) = &cv.unit_name {
                // name-only fallback: try each sub-type
                if let Some(u) = TimeUnit::from_name(name) {
                    return Some(Self::Time(u));
                }
                if let Some(u) = IntensityUnit::from_name(name) {
                    return Some(Self::Intensity(u));
                }
                if let Some(u) = EnergyUnit::from_name(name) {
                    return Some(Self::Energy(u));
                }
                if name == "m/z" {
                    return Some(Self::MassToCharge);
                }
            }
        }
        None
    }
}

// ── Top-level helpers ─────────────────────────────────────────────────────────

/// Convert a scalar CVParam value to an f32 Time quantity.
/// `default` is used when the CVParam carries no unit information.
pub fn cv_to_time_f32(
    cv: &ControlledVocabularyParameter,
    default: TimeUnit,
) -> Option<uom::si::f32::Time> {
    let value: f32 = cv.value.parse().ok()?;
    let unit = TimeUnit::try_from(cv).unwrap_or(default);
    Some(unit.to_quantity_f32(value))
}

/// Convert a scalar CVParam value to an f64 Time quantity.
/// `default` is used when the CVParam carries no unit information.
pub fn cv_to_time_f64(cv: &ControlledVocabularyParameter, default: TimeUnit) -> Option<Time> {
    let value: f64 = cv.value.parse().ok()?;
    let unit = TimeUnit::try_from(cv).unwrap_or(default);
    Some(unit.to_quantity_f64(value))
}

/// Convert a scalar CVParam value to an f64 Energy quantity.
pub fn cv_to_energy_f64(cv: &ControlledVocabularyParameter) -> Option<Energy> {
    let value: f64 = cv.value.parse().ok()?;
    EnergyUnit::try_from(cv)
        .ok()
        .map(|u| u.to_quantity_f64(value))
}

/// Convert a scalar CVParam value to an MzValue (MS:1000040 / unit_name "m/z").
pub fn cv_to_mz(cv: &ControlledVocabularyParameter) -> Option<MzValue> {
    let is_mz = cv.unit_accession.as_deref() == Some("MS:1000040")
        || cv.unit_name.as_deref() == Some("m/z");
    if !is_mz {
        return None;
    }
    cv.value.parse().ok().map(MzValue)
}
