use winnow::{
    ascii::{line_ending, multispace0, multispace1},
    combinator::{alt, eof, preceded, repeat, separated_pair, terminated},
    stream::AsChar,
    token::take_till,
    PResult, Parser,
};

#[inline]
fn kv<'a>(input: &mut &'a str) -> PResult<(&'a str, &'a str)> {
    separated_pair(key, separator, right).parse_next(input)
}

#[inline]
fn key<'a>(input: &mut &'a str) -> PResult<&'a str> {
    alt(("machine", "login", "password")).parse_next(input)
}

#[inline]
fn separator<'a>(input: &mut &'a str) -> PResult<()> {
    multispace1.void().parse_next(input)
}

#[inline]
fn right<'a>(input: &mut &'a str) -> PResult<&'a str> {
    terminated(
        take_till(1.., |c: char| c.is_whitespace()).verify(|s: &str| !s.starts_with('#')),
        multispace0,
    )
    .parse_next(input)
}

#[inline]
fn comment<'a>(input: &mut &'a str) -> PResult<()> {
    ('#', line_reset).void().parse_next(input)
}

#[inline]
fn line_reset<'a>(input: &mut &'a str) -> PResult<()> {
    (take_till(0.., |c: char| c.is_newline()), line_ending)
        .void()
        .parse_next(input)
}

#[inline]
fn whitespace<'a>(input: &mut &'a str) -> PResult<()> {
    multispace1.void().parse_next(input)
}

#[inline]
fn garbage<'a>(input: &mut &'a str) -> PResult<()> {
    alt((whitespace, comment)).parse_next(input)
}

#[inline]
fn multi_ignore<'a>(input: &mut &'a str) -> PResult<()> {
    repeat(0.., garbage).parse_next(input)
}

#[inline]
pub(crate) fn multiline<'a>(input: &mut &'a str) -> PResult<Vec<Vec<(&'a str, &'a str)>>> {
    repeat(0.., preceded(multi_ignore, line)).parse_next(input)
}

#[inline]
pub(crate) fn line<'a>(input: &mut &'a str) -> PResult<Vec<(&'a str, &'a str)>> {
    fn verify_line((res, ()): &(Vec<(&str, &str)>, ())) -> bool {
        for i in ["machine", "login", "password"] {
            if !res.iter().any(|x| x.0 == i) {
                return false;
            }
        }

        true
    }

    let (res, _) = (
        repeat(3, kv),
        alt((line_ending.void(), eof.void(), comment)),
    )
        .verify(verify_line)
        .parse_next(input)?;

    Ok(res)
}


#[test]
fn test_garbage() {
    let a = "# 123\n";
    let b = "#123\n";
    let c = "\n";
    let d = "#\n";
    let e = "    \n";
    let f = "    ";

    for mut i in [a, b, c, d, e, f] {
        let out = garbage(&mut i);
        assert!(out.is_ok());
        assert!(i.is_empty());
    }
}

#[test]
fn test_multi_garbage() {
    let a = "\n# 123\n";
    let b = "# 123\n\n";
    let c = "\n\n# 123";

    for mut i in [a, b, c] {
        let out = multi_ignore(&mut i);
        assert!(out.is_ok());
        dbg!(i);
    }
}

#[test]
fn test_single_line() {
    let s = "machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq #sdadasdas\n";
    let s2 = "machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq\n";
    let s3 = "machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq";
    let s4 = "machine esm.ubuntu.com/apps/ubuntu/ password qaq login bearer";
    let s5 = "machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq # sdadasdas\n";
    for mut i in [s, s2, s3, s4, s5] {
        let mut l = line(&mut i).unwrap();
        l.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            l,
            vec![
                ("login", "bearer"),
                ("machine", "esm.ubuntu.com/apps/ubuntu/"),
                ("password", "qaq")
            ]
        );
        assert_eq!(i, "");
    }

    let mut i = "machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq sdadasdas\n";
    let l = line(&mut i);
    assert!(l.is_err());

    let mut i = "machine esm.ubuntu.com/apps/ubuntu/ # password qaq login bearer";
    let l = line(&mut i);
    assert!(l.is_err());

    let mut i = "machine #esm.ubuntu.com/apps/ubuntu/ password qaq login bearer";
    let l = line(&mut i);
    assert!(l.is_err());
}

#[test]
fn test_multi_line() {
    let config1 = r#"# 123
machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq # ubuntu-pro-client
machine esm.ubuntu.com/infra/ubuntu/ login bearer password qaq # ubuntu-pro-client
"#;
    let config2 = r#"
#123
machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq # ubuntu-pro-client
machine esm.ubuntu.com/infra/ubuntu/ login bearer password qaq # ubuntu-pro-client
"#;
    let config3 = r#"
machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq # ubuntu-pro-client
machine esm.ubuntu.com/infra/ubuntu/ login bearer password qaq # ubuntu-pro-client
"#;
    let config4 = r#"
machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq # ubuntu-pro-client
#123
machine esm.ubuntu.com/infra/ubuntu/ login bearer password qaq # ubuntu-pro-client
"#;
    let config5 = r#"
machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq # ubuntu-pro-client
machine esm.ubuntu.com/infra/ubuntu/ login bearer password qaq # ubuntu-pro-client
#123
"#;
    let config6 = r#"
machine esm.ubuntu.com/apps/ubuntu/ login bearer password qaq # ubuntu-pro-client
machine esm.ubuntu.com/infra/ubuntu/ login bearer password qaq # ubuntu-pro-client

#123"#;

    for mut i in [config1, config2, config3, config4, config5, config6] {
        let out = multiline(&mut i);
        assert_eq!(
            out,
            Ok(vec![
                vec![
                    ("machine", "esm.ubuntu.com/apps/ubuntu/",),
                    ("login", "bearer",),
                    ("password", "qaq",),
                ],
                vec![
                    ("machine", "esm.ubuntu.com/infra/ubuntu/",),
                    ("login", "bearer",),
                    ("password", "qaq",),
                ],
            ],)
        );
    }
}
