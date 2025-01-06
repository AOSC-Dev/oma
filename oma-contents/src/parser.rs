use winnow::{combinator::separated, token::take_till, PResult, Parser};

use crate::OmaContentsError;

/// Parse Contents file from input
pub fn parse_contetns(input: &str) -> Result<Vec<(&str, Vec<&str>)>, OmaContentsError> {
    input
        .lines()
        .enumerate()
        .map(|x| match parse_contents_single_line(x.1) {
            Ok(v) => Ok(v),
            Err(e) => Err(OmaContentsError::InvalidContentsWithLine(
                e.to_string(),
                x.0,
            )),
        })
        .collect::<Result<Vec<_>, OmaContentsError>>()
}

/// Parse single line Contents
pub fn parse_contents_single_line(input: &str) -> Result<(&str, Vec<&str>), OmaContentsError> {
    // https://wiki.debian.org/DebianRepository/Format#A.22Contents.22_indices
    // 最后一个空格是分隔符
    let (file, pkgs) = input
        .rsplit_once(|c: char| c.is_whitespace() && c != '\n')
        .ok_or(OmaContentsError::InvalidContents(
            "Failed to get last space".to_string(),
        ))?;

    let file = file.trim();
    let mut pkgs = pkgs.trim();
    let pkgs = multi_packages(&mut pkgs)
        .map_err(|e| OmaContentsError::InvalidContents(format!("Failed to get packages: {e}")))?;

    Ok((file, pkgs))
}

#[inline]
fn single_package<'a>(input: &mut &'a str) -> PResult<&'a str> {
    take_till(0.., |c| c == ',' || c == '\n').parse_next(input)
}

#[inline]
fn multi_packages<'a>(input: &mut &'a str) -> PResult<Vec<&'a str>> {
    separated(0.., single_package, ',').parse_next(input)
}

#[test]
fn test_single_line() {
    let s = "etc/dpkg/dpkg.cfg.d/pk4\t\t\t\t\t    universe/utils/pk4\n";
    let res = parse_contents_single_line(s);

    assert_eq!(
        res.unwrap(),
        ("etc/dpkg/dpkg.cfg.d/pk4", vec!["universe/utils/pk4"])
    )
}

#[test]
fn test_single_line_multi_packages() {
    let s = "opt/32/libexec   devel/gcc+32,devel/llvm+32,gnome/gconf+32,libs/gdk-pixbuf+32\n";
    let res = parse_contents_single_line(s);

    assert_eq!(
        res.unwrap(),
        (
            "opt/32/libexec",
            vec![
                "devel/gcc+32",
                "devel/llvm+32",
                "gnome/gconf+32",
                "libs/gdk-pixbuf+32"
            ]
        )
    )
}

#[test]
fn test_single_line_file_multi_space() {
    let s = "/etc/i have multi space foo/bar/abc\n";
    let res = parse_contents_single_line(s);

    assert_eq!(
        res.unwrap(),
        ("/etc/i have multi space", vec!["foo/bar/abc"])
    )
}
