use clap_mangen::Man;

include!("src/args.rs");

fn main() -> std::io::Result<()> {
    let cmd = command_builder();

    let man = Man::new(cmd.clone());
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


    for subcommand in cmd.get_subcommands() {
        let subcommand_name = format!("oma-{}", subcommand.get_name());
        let mut buffer: Vec<u8> = Default::default();
        let man = Man::new(subcommand.clone()).title(&subcommand_name);
        man.render(&mut buffer)?;
        std::fs::write(
            std::path::PathBuf::from(&man_dir).join(format!("{}{}", &subcommand_name, ".1")),
            buffer,
        )?;
    }

    Ok(())
}
