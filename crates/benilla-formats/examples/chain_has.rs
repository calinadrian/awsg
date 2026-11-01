//! Development helper: `chain_has <data-dir> <paths-file>` prints `1 <path>` or `0 <path>` for
//! each line of the paths file, whether the patch chain can read it.

use std::path::Path;

use benilla_formats::Chain;

fn main() {
    let mut args = std::env::args().skip(1);
    let (data, list) = (
        args.next().expect("data dir"),
        args.next().expect("paths file"),
    );
    let chain = Chain::open(Path::new(&data)).expect("open patch chain");
    let text = std::fs::read_to_string(list).expect("read paths file");
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        println!("{} {}", u8::from(chain.contains(line)), line);
    }
}
