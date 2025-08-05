use oma_mirror::parser::{MirrorsConfig, TemplateParseError};

fn main() -> Result<(), TemplateParseError> {
    let m = MirrorsConfig::parse_from_file("./oma-mirror/examples/mirror_config/1.toml")?;
    dbg!(m);

    Ok(())
}
