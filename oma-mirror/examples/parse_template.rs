use std::path::Path;

use oma_mirror::parser::{MirrorsConfigTemplate, TemplateParseError};

fn main() -> Result<(), TemplateParseError> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p1 = dir.join("examples/mirror_templates/1.toml");
    let p2 = dir.join("examples/mirror_templates/2.toml");

    let t = MirrorsConfigTemplate::parse_from_file(p1)?;
    dbg!(t);

    let t = MirrorsConfigTemplate::parse_from_file(p2)?;
    dbg!(t);

    Ok(())
}
