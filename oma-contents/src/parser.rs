use winnow::{
    combinator::{separated, separated_pair},
    error::ParserError,
    token::{take_till, take_until},
    PResult, Parser,
};

#[inline]
fn pkg_split<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<(&'a str, &'a str), E> {
    separated_pair(take_till(0.., '/'), pkg_name_sep, second_single).parse_next(input)
}

#[inline]
fn pkg_name_sep<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<(), E> {
    "/".void().parse_next(input)
}

#[inline]
fn second_single<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<&'a str, E> {
    take_till(0.., |c| c == ',' || c == '\n').parse_next(input)
}

#[inline]
fn second<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<Vec<(&'a str, &'a str)>, E> {
    separated(0.., pkg_split, ',').parse_next(input)
}

#[inline]
fn first<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<&'a str, E> {
    take_until(1.., "   ").parse_next(input)
}

#[inline]
fn sep<'a, E: ParserError<&'a str>>(input: &mut &'a str) -> PResult<(), E> {
    "   ".void().parse_next(input)
}

type ContentsLine<'a> = (&'a str, Vec<(&'a str, &'a str)>);

#[inline]
pub fn single_line<'a, E: ParserError<&'a str>>(
    input: &mut &'a str,
) -> PResult<ContentsLine<'a>, E> {
    separated_pair(first, sep, second).parse_next(input)
}

pub type ContentsLines<'a> = Vec<(&'a str, Vec<(&'a str, &'a str)>)>;

#[inline]
pub fn parse_contents<'a, E: ParserError<&'a str>>(
    input: &mut &'a str,
) -> PResult<ContentsLines<'a>, E> {
    use winnow::combinator::{repeat, terminated};
    repeat(1.., terminated(single_line, "\n")).parse_next(input)
}

#[test]
fn test_pkg_name() {
    let a = &mut "admin/apt-file\n/   qaq/qaq\n";
    let res = pkg_split::<()>(a);

    assert_eq!(res, Ok(("admin", "apt-file")))
}

#[test]
fn test_second_single() {
    let a = &mut "admin/apt-file\n/   qaq/qaq\n";
    let res = second_single::<()>(a);
    assert_eq!(res, Ok("admin/apt-file"));

    let b = &mut "admin/apt-file";
    let res = second_single::<()>(b);
    assert_eq!(res, Ok("admin/apt-file"));
}

#[test]
fn test_second() {
    let a = &mut "admin/apt-file,admin/apt\n";
    let res = second::<()>(a);
    assert_eq!(res, Ok(vec![("admin", "apt-file"), ("admin", "apt")]));

    let b = &mut "admin/apt-file\n";
    let res = second::<()>(b);
    assert_eq!(res, Ok(vec![("admin", "apt-file")]));
}

#[test]
fn test_single_line() {
    let a = &mut "opt/32/libexec   devel/gcc+32,devel/llvm+32,gnome/gconf+32,libs/gdk-pixbuf+32\nopt/32/share   devel/llvm+32,libs/alsa-plugins+32,x11/libxkbcommon+32,x11/mesa+32,x11/virtualgl+32";
    let res = single_line::<()>(a);
    assert_eq!(
        res,
        Ok((
            "opt/32/libexec",
            vec![
                ("devel", "gcc+32"),
                ("devel", "llvm+32"),
                ("gnome", "gconf+32"),
                ("libs", "gdk-pixbuf+32")
            ]
        ))
    );

    let b = &mut "/   admin/apt-file\n";
    let res = single_line::<()>(b);
    assert_eq!(res, Ok(("/", vec![("admin", "apt-file")])));
}

#[test]
fn test_multiple_lines() {
    let a = &mut "opt/32/libexec   devel/gcc+32,devel/llvm+32,gnome/gconf+32\nopt/32/share   devel/llvm+32,libs/alsa-plugins+32\n";
    let res = parse_contents::<()>(a);

    assert_eq!(
        res,
        Ok(vec![
            (
                "opt/32/libexec",
                vec![
                    ("devel", "gcc+32"),
                    ("devel", "llvm+32"),
                    ("gnome", "gconf+32"),
                ]
            ),
            (
                "opt/32/share",
                vec![("devel", "llvm+32"), ("libs", "alsa-plugins+32")]
            )
        ])
    )
}
