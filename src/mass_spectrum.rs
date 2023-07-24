pub trait MassScan {
    ///Return retention time in minutes.
    fn rt(&self) -> Option<uom::si::f32::Time>;
    fn ms_level(&self) -> Option<u16>;
}
pub trait MassSpectrum {
    type Err;
    fn peaks(&self) -> Result<Vec<(f64, f64)>, Self::Err>;
}
