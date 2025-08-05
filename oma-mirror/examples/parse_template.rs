use oma_mirror::parser::{MirrorsConfigTemplate, TemplateParseError};

fn main() -> Result<(), TemplateParseError> {
    let t =
        MirrorsConfigTemplate::parse_from_file("./oma-mirror/examples/mirror_templates/1.toml")?;
    dbg!(t);

    let t =
        MirrorsConfigTemplate::parse_from_file("./oma-mirror/examples/mirror_templates/2.toml")?;
    dbg!(t);

    Ok(())
}
