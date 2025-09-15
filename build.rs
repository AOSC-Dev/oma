use std::error::Error;

use vergen::{CargoBuilder, Emitter};
use vergen_git2::Git2Builder;

fn main() -> Result<(), Box<dyn Error>> {
    let git = Git2Builder::default()
        .sha(true)
        .dirty(true)
        .describe(true, false, None)
        .build()?;

    let cargo = CargoBuilder::default().features(true).debug(true).build()?;

    Emitter::default()
        .add_instructions(&git)?
        .add_instructions(&cargo)?
        .emit()?;

    Ok(())
}
