fn main() {
    let apt_config_dir = "apt-config-grammar";
    let src_dir = format!("{apt_config_dir}/src");

    cc::Build::new()
        .file(format!("{src_dir}/parser.c"))
        .include(&src_dir)
        .compile("tree-sitter-apt-config");

    println!("cargo::rerun-if-changed={src_dir}/parser.c");
    println!("cargo::rerun-if-changed={src_dir}/tree_sitter/parser.h");
}
