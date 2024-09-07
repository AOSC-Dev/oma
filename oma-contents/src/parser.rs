pub(crate) fn single_line(input: &str) -> Option<(String, Vec<&str>)> {
    // https://wiki.debian.org/DebianRepository/Format#A.22Contents.22_indices
    let mut s = input.split_whitespace();
    // 最后一个空格是分隔符
    let pkgs = s.next_back()?;
    let file = s.collect::<Vec<_>>().join(" ");
    let pkgs = pkgs.split(',').collect::<Vec<_>>();

    Some((file, pkgs))
}

#[test]
fn test_single_line() {
    let s = "etc/dpkg/dpkg.cfg.d/pk4\t\t\t\t\t    universe/utils/pk4\n";
    let res = single_line(s);

    assert_eq!(
        res,
        Some((
            "etc/dpkg/dpkg.cfg.d/pk4".to_string(),
            vec!["universe/utils/pk4"]
        ))
    )
}

#[test]
fn test_single_line_multi_packages() {
    let s = "opt/32/libexec   devel/gcc+32,devel/llvm+32,gnome/gconf+32,libs/gdk-pixbuf+32\n";
    let res = single_line(s);

    assert_eq!(
        res,
        Some((
            "opt/32/libexec".to_string(),
            vec![
                "devel/gcc+32",
                "devel/llvm+32",
                "gnome/gconf+32",
                "libs/gdk-pixbuf+32"
            ]
        ))
    )
}

#[test]
fn test_single_line_file_multi_space() {
    let s = "/etc/i have multi space foo/bar/abc\n";
    let res = single_line(s);

    assert_eq!(
        res,
        Some((
            "/etc/i have multi space".to_string(),
            vec![
                "foo/bar/abc"
            ]
        ))
    )
}
