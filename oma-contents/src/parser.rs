use winnow::{combinator::separated, token::take_till, PResult, Parser};

use crate::OmaContentsError;

/// Parse Contents file from input
pub fn parse_contetns(input: &str) -> Result<Vec<(&str, Vec<&str>)>, OmaContentsError> {
    input
        .lines()
        .map(single_line)
        .collect::<Option<Vec<_>>>()
        .ok_or(OmaContentsError::InvaildContents)
}

pub(crate) fn single_line(input: &str) -> Option<(&str, Vec<&str>)> {
    // https://wiki.debian.org/DebianRepository/Format#A.22Contents.22_indices
    // 最后一个空格是分隔符
    let (file, pkgs) = input.rsplit_once(|c: char| c.is_whitespace() && c != '\n')?;
    let file = file.trim();
    let mut pkgs = pkgs.trim();
    let pkgs = multi_packages(&mut pkgs).ok()?;

    Some((file, pkgs))
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
    let res = single_line(s);

    assert_eq!(
        res,
        Some(("etc/dpkg/dpkg.cfg.d/pk4", vec!["universe/utils/pk4"]))
    )
}

#[test]
fn test_single_line_multi_packages() {
    let s = "opt/32/libexec   devel/gcc+32,devel/llvm+32,gnome/gconf+32,libs/gdk-pixbuf+32\n";
    let res = single_line(s);

    assert_eq!(
        res,
        Some((
            "opt/32/libexec",
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

    assert_eq!(res, Some(("/etc/i have multi space", vec!["foo/bar/abc"])))
}
