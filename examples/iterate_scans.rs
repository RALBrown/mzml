use mzml::mass_spectrum::MassScan;
use mzml::LazyMzML;
use std::fs::File;

fn main() {
    let file =
        File::open(r"C:\Users\Robert\Documents\GitHub\mzml\test_data\small.pwiz.1.1.mzML").unwrap();
    let mzml_struct = LazyMzML::new(file).unwrap();
    for spectrum in mzml_struct.iter_scan() {
        println!(
            "{:.2}\t\t{:?}",
            spectrum
                .rt()
                .unwrap()
                .into_format_args(uom::si::time::second, uom::fmt::DisplayStyle::Abbreviation),
            spectrum.ion_fill_time()
        );
    }
}
