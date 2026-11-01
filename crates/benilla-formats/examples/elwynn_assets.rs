//! Named-asset pipe for scripts/package_elwynn.py; uses the runtime's exact patch priority.
use std::io::{self, BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result};
use benilla_formats::Chain;

fn main() -> Result<()> {
    let data = std::env::args()
        .nth(1)
        .context("expected source Data directory")?;
    let chain = Chain::open(Path::new(&data))?;
    let mut output = io::BufWriter::new(io::stdout().lock());
    for request in io::stdin().lock().lines() {
        let request = request?;
        let result = if request == "@list" {
            chain.list().map(|entries| {
                entries
                    .into_iter()
                    .map(|e| format!("{}\t{}\n", e.size, e.name))
                    .collect::<String>()
                    .into_bytes()
            })
        } else {
            chain.read(&request)
        };
        match result {
            Ok(bytes) => {
                output.write_all(&(bytes.len() as i64).to_le_bytes())?;
                output.write_all(&bytes)?;
            }
            Err(error) => {
                let message = format!("{error:#}");
                output.write_all(&(-(message.len() as i64)).to_le_bytes())?;
                output.write_all(message.as_bytes())?;
            }
        }
        output.flush()?;
    }
    Ok(())
}
