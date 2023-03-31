include!("src/args.rs");

fn main() -> std::io::Result<()> {
    let cmd = command_builder();

    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    let man_dir = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").ok_or(std::io::ErrorKind::NotFound)?,
    )
    .join("man");

    if !man_dir.is_dir() {
        std::fs::create_dir_all(&man_dir)?;
    }

    std::fs::write(man_dir.join("oma.1"), buffer)?;

    Ok(())
}
