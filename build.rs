// use clap::CommandFactory;
// use clap_complete::{generate_to, Shell};
// use clap_mangen::Man;
// use std::io::Result;
// use std::path::Path;

// include!("src/args_v2.rs");

// const GENERATED_COMPLETIONS: &[Shell] = &[Shell::Bash, Shell::Zsh, Shell::Fish];

// fn main() -> Result<()> {
//     let srcdir = std::path::PathBuf::from(
//         std::env::var_os("CARGO_MANIFEST_DIR").ok_or(std::io::ErrorKind::NotFound)?,
//     );

//     let cmd = OhManagerAilurus::command();
//     build_man(&cmd, &srcdir)?;
//     build_completions(srcdir, cmd)?;

//     Ok(())
// }

// fn build_completions(srcdir: std::path::PathBuf, mut cmd: Command) -> Result<()> {
//     println!("cargo:rerun-if-env-changed=OMA_GEN_COMPLETIONS");

//     if std::env::var("OMA_GEN_COMPLETIONS").is_ok() {
//         std::fs::create_dir_all(srcdir.join("completions"))?;

//         for shell in GENERATED_COMPLETIONS {
//             generate_to(*shell, &mut cmd, "oma", "completions")
//                 .expect("Failed to generate shell completions");
//         }
//     }

//     Ok(())
// }

// fn build_man(cmd: &Command, srcdir: &Path) -> Result<()> {
//     let man = Man::new(cmd.clone());
//     let mut buffer: Vec<u8> = Default::default();
//     man.render(&mut buffer)?;

//     let man_dir = srcdir.join("man");
//     if !man_dir.is_dir() {
//         std::fs::create_dir_all(&man_dir)?;
//     }

//     std::fs::write(man_dir.join("oma.1"), buffer)?;

//     for subcommand in cmd.get_subcommands() {
//         if subcommand.is_hide_set() {
//             continue;
//         }

//         let subcommand_name = format!("oma-{}", subcommand.get_name());
//         let mut buffer: Vec<u8> = Default::default();

//         let man = Man::new(subcommand.clone()).title(&subcommand_name);
//         man.render(&mut buffer)?;

//         std::fs::write(
//             std::path::PathBuf::from(&man_dir).join(format!("{subcommand_name}.1")),
//             buffer,
//         )?;
//     }

//     Ok(())
// }

fn main() {}
