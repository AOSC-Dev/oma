use clap_complete::{generate_to, Shell};
use clap_mangen::Man;

include!("src/args.rs");

const GENERATED_COMPLETIONS: &[Shell] = &[Shell::Bash, Shell::Zsh, Shell::Fish];

fn main() -> std::io::Result<()> {
    let mut cmd = command_builder();

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
        if subcommand.is_hide_set() {
            continue;
        }
        let subcommand_name = format!("oma-{}", subcommand.get_name());
        let mut buffer: Vec<u8> = Default::default();
        let man = Man::new(subcommand.clone()).title(&subcommand_name);
        man.render(&mut buffer)?;
        std::fs::write(
            std::path::PathBuf::from(&man_dir).join(format!("{}{}", &subcommand_name, ".1")),
            buffer,
        )?;
    }

    println!("cargo:rerun-if-env-changed=CIEL_GEN_COMPLETIONS");

    // generate completions on demand
    if std::env::var("CIEL_GEN_COMPLETIONS").is_ok() {
        let p = std::path::PathBuf::from(
            std::env::var_os("CARGO_MANIFEST_DIR").ok_or(std::io::ErrorKind::NotFound)?,
        );

        std::fs::create_dir_all(p.join("completions"))?;

        for shell in GENERATED_COMPLETIONS {
            generate_to(*shell, &mut cmd, "oma", "completions")
                .expect("Failed to generate shell completions");
        }
    }

    Ok(())
}
