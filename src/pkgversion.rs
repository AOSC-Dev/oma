use anyhow::{bail, format_err, Result};
use lazy_static::lazy_static;
use log::warn;
use nom::{
    character::complete::*,
    error::{context, ErrorKind},
    sequence::*,
    IResult, InputTakeAtPosition,
};
use serde::{Deserialize, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;

lazy_static! {
    static ref DIGIT_TABLE: Vec<char> = "1234567890".chars().collect();
    static ref NON_DIGIT_TABLE: Vec<char> =
        "~|ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz+-."
            .chars()
            .collect();
}

/// dpkg style version comparison.
#[derive(PartialEq, Eq, Clone, Debug, Deserialize)]
pub struct PkgVersion {
    pub epoch: usize,
    pub version: Vec<(String, Option<u128>)>,
    pub revision: usize,
}

fn is_upstream_version_char(c: char) -> bool {
    c.is_alphanumeric() || c == '.' || c == '+' || c == '~'
}

fn upstream_version(i: &str) -> IResult<&str, &str> {
    i.split_at_position1_complete(|item| !is_upstream_version_char(item), ErrorKind::Char)
}

/// The standard AOSC package version parser
/// See more at https://wiki.aosc.io/developer/packaging/package-styling-manual/#versioning-variables
fn standard_parse_version(i: &str) -> IResult<&str, PkgVersion> {
    let (i, epoch) = match context(
        "Parsing epoch...",
        pair::<_, _, _, nom::error::Error<&str>, _, _>(digit1, char(':')),
    )(i)
    {
        Ok((i, (epoch, _))) => (i, epoch.parse().unwrap()),
        Err(_) => (i, 0),
    };
    let (i, upstream_version) = context("Parsing upstream_version...", upstream_version)(i)?;
    let (i, revision) = match context(
        "Parsing revision...",
        pair::<_, _, _, nom::error::Error<&str>, _, _>(char('-'), digit1),
    )(i)
    {
        Ok((i, (_, revision))) => (i, revision.parse().unwrap()),
        Err(_) => (i, 0),
    };

    let res = PkgVersion {
        epoch,
        version: parse_version_string(upstream_version).unwrap(),
        revision,
    };

    Ok((i, res))
}

fn alt_is_upstream_version_char(c: char) -> bool {
    is_upstream_version_char(c) || c == '-'
}

fn alt_upstream_version(i: &str) -> IResult<&str, &str> {
    i.split_at_position1_complete(|item| !alt_is_upstream_version_char(item), ErrorKind::Char)
}

/// Alternative version parser, allowing '-' in upstream_version and assume no revision
/// TODO: For compatibilities only. Remove me!
fn alt_parse_version(i: &str) -> IResult<&str, PkgVersion> {
    let (i, epoch) = match context(
        "Parsing epoch...",
        nom::sequence::pair::<_, _, _, nom::error::Error<&str>, _, _>(digit1, char(':')),
    )(i)
    {
        Ok((i, (epoch, _))) => (i, epoch.parse().unwrap()),
        Err(_) => (i, 0),
    };
    let (i, upstream_version) = context("Parsing upstream_version...", alt_upstream_version)(i)?;

    let res = PkgVersion {
        epoch,
        version: parse_version_string(upstream_version).unwrap(),
        revision: 0,
    };

    Ok((i, res))
}

/// A public interface for parsing versions. Will try to parse it with standard first.
/// If that doesn't work, parse it with compatible method
pub fn parse_version(i: &str) -> IResult<&str, PkgVersion> {
    let (i, res) = match context("parsing PkgVersion...", standard_parse_version)(i) {
        Ok(res) => res,
        Err(_) => {
            warn!(
                "Unable to parse version {} with standard parser, switching to compatible mode...",
                i
            );
            context(
                "Parsing PkgVersion with compatible mode...",
                alt_parse_version,
            )(i)?
        }
    };
    Ok((i, res))
}

impl TryFrom<&str> for PkgVersion {
    type Error = anyhow::Error;
    fn try_from(s: &str) -> Result<Self> {
        let (_, res) = parse_version(s).map_err(|e| format_err!("Malformed version: {} .", e))?;
        Ok(res)
    }
}

impl fmt::Display for PkgVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.epoch != 0 {
            write!(f, "{}:", self.epoch)?;
        }
        for segment in &self.version {
            write!(f, "{}", &segment.0)?;
            if let Some(num) = segment.1 {
                write!(f, "{num}")?;
            }
        }
        if self.revision != 0 {
            write!(f, "-{}", self.revision)?;
        }
        Ok(())
    }
}

impl Serialize for PkgVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let res = self.to_string();
        serializer.serialize_str(&res)
    }
}

impl Ord for PkgVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.epoch > other.epoch {
            return Ordering::Greater;
        }

        if self.epoch < other.epoch {
            return Ordering::Less;
        }

        let mut self_vec = self.version.clone();
        let mut other_vec = other.version.clone();
        // Add | to the back to make sure end of string is more significant than '~'
        self_vec.push(("|".to_string(), None));
        other_vec.push(("|".to_string(), None));
        // Reverse them so that we can pop them
        self_vec.reverse();
        other_vec.reverse();
        while !self_vec.is_empty() {
            // Match non digit
            let mut x = self_vec.pop().unwrap();
            let mut y = match other_vec.pop() {
                Some(y) => y,
                None => {
                    return Ordering::Greater;
                }
            };

            // Magic! To make sure end of string have the correct rank
            x.0.push('|');
            y.0.push('|');
            let x_nondigit_rank = str_to_ranks(&x.0);
            let y_nondigit_rank = str_to_ranks(&y.0);
            for (pos, r_x) in x_nondigit_rank.iter().enumerate() {
                match r_x.cmp(&y_nondigit_rank[pos]) {
                    Ordering::Greater => {
                        return Ordering::Greater;
                    }
                    Ordering::Less => {
                        return Ordering::Less;
                    }
                    Ordering::Equal => (),
                }
            }

            // Compare digit part
            let x_digit = x.1.unwrap_or(0);
            let y_digit = y.1.unwrap_or(0);
            match x_digit.cmp(&y_digit) {
                Ordering::Greater => {
                    return Ordering::Greater;
                }
                Ordering::Less => {
                    return Ordering::Less;
                }
                Ordering::Equal => (),
            }
        }

        // If other still has remaining segments
        if !other_vec.is_empty() {
            return Ordering::Greater;
        }

        // Finally, compare revision
        match self.revision.cmp(&other.revision) {
            Ordering::Greater => {
                return Ordering::Greater;
            }
            Ordering::Less => {
                return Ordering::Less;
            }
            Ordering::Equal => (),
        }

        Ordering::Equal
    }
}

impl PartialOrd for PkgVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_version_string(s: &str) -> Result<Vec<(String, Option<u128>)>> {
    if s.is_empty() {
        bail!("Empty version string.")
    }

    if !s.starts_with(|c: char| c.is_ascii_digit()) {
        bail!("Version string must start with a digit.")
    }

    let mut in_digit = true;
    let mut nondigit_buffer = String::new();
    let mut digit_buffer = String::new();
    let mut result = Vec::new();
    for c in s.chars() {
        if NON_DIGIT_TABLE.contains(&c) {
            if in_digit && !digit_buffer.is_empty() {
                // Previously in digit sequence
                // Try to parse digit segment
                let num: u128 = digit_buffer.parse()?;
                result.push((nondigit_buffer.clone(), Some(num)));
                nondigit_buffer.clear();
                digit_buffer.clear();
            }
            nondigit_buffer.push(c);
            in_digit = false;
        } else if DIGIT_TABLE.contains(&c) {
            digit_buffer.push(c);
            in_digit = true;
        } else {
            // This should not happen, we should have sanitized input
            bail!("Invalid character in version string.")
        }
    }

    // Commit last segment
    if digit_buffer.is_empty() {
        result.push((nondigit_buffer, None));
    } else {
        result.push((nondigit_buffer, Some(digit_buffer.parse::<u128>()?)));
    }
    Ok(result)
}

fn str_to_ranks(s: &str) -> Vec<usize> {
    let res: Vec<usize> = s
        .chars()
        .map(|c| {
            // Input should already be sanitized with the input regex
            NON_DIGIT_TABLE.iter().position(|&i| c == i).unwrap()
        })
        .collect();

    res
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let source = "22.04.3";
        let v = PkgVersion::try_from(source);
        dbg!(v.unwrap().to_string());
    }

    #[test]
    fn pkg_ver_from_str() {
        let source = vec!["22.04.1.", "999:0+git20210608-1"];
        let result = vec![
            PkgVersion {
                epoch: 0,
                version: vec![
                    ("".to_string(), Some(1)),
                    (".".to_string(), Some(1)),
                    (".".to_string(), Some(1)),
                    (".".to_string(), None),
                ],
                revision: 0,
            },
            PkgVersion {
                epoch: 999,
                version: vec![
                    ("".to_string(), Some(0)),
                    ("+git".to_string(), Some(20210608)),
                ],
                revision: 1,
            },
        ];

        for (pos, e) in source.iter().enumerate() {
            assert_eq!(PkgVersion::try_from(*e).unwrap(), result[pos]);
        }
    }

    use std::cmp::Ordering::*;
    #[test]
    fn pkg_ver_ord() {
        let source = vec![
            ("1.1.1", Less, "1.1.2"),
            ("1b", Greater, "1a"),
            ("1~~", Less, "1~~a"),
            ("1~~a", Less, "1~"),
            ("1", Less, "1.1"),
            ("1.0", Less, "1.1"),
            ("1.2", Less, "1.11"),
            ("1.0-1", Less, "1.1"),
            ("1.0-1", Less, "1.0-12"),
            // make them different for sorting
            ("1:1.0-0", Equal, "1:1.0"),
            ("1.0", Equal, "1.0"),
            ("1.0-1", Equal, "1.0-1"),
            ("1:1.0-1", Equal, "1:1.0-1"),
            ("1:1.0", Equal, "1:1.0"),
            ("1.0-1", Less, "1.0-2"),
            //("1.0final-5sarge1", Greater, "1.0final-5"),
            ("1.0final-5", Greater, "1.0a7-2"),
            ("0.9.2-5", Less, "0.9.2+cvs.1.0.dev.2004.07.28-1"),
            ("1:500", Less, "1:5000"),
            ("100:500", Greater, "11:5000"),
            ("1.0.4-2", Greater, "1.0pre7-2"),
            ("1.5~rc1", Less, "1.5"),
            ("1.5~rc1", Less, "1.5+1"),
            ("1.5~rc1", Less, "1.5~rc2"),
            ("1.5~rc1", Greater, "1.5~dev0"),
        ];

        for e in source {
            println!("Comparing {} vs {}", e.0, e.2);
            println!(
                "{:#?} vs {:#?}",
                PkgVersion::try_from(e.0).unwrap(),
                PkgVersion::try_from(e.2).unwrap()
            );
            assert_eq!(
                PkgVersion::try_from(e.0)
                    .unwrap()
                    .cmp(&PkgVersion::try_from(e.2).unwrap()),
                e.1
            );
        }
    }

    #[test]
    fn pkg_ver_eq() {
        let source = vec![("1.1+git2021", "1.1+git2021")];
        for e in &source {
            assert_eq!(
                PkgVersion::try_from(e.0).unwrap(),
                PkgVersion::try_from(e.1).unwrap()
            );
        }
    }
}
