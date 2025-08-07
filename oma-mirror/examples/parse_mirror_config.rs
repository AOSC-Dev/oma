use std::path::Path;

use oma_mirror::parser::{MirrorsConfig, TemplateParseError};

fn main() -> Result<(), TemplateParseError> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let m = MirrorsConfig::parse_from_file(dir.join("examples/mirror_config/1.toml"))?;
    dbg!(m);

    Ok(())
}
