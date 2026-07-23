//! Parsing of deb822 dependency list fields (Depends, Recommends, Provides, etc.).

use winnow::{
    ModalResult, Parser,
    ascii::{multispace0, space0, space1},
    combinator::{alt, delimited, opt, preceded, separated},
    token::{literal, take_till},
};

/// The version constraint operator in a dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// << (strictly less than)
    Lt,
    /// <= (less than or equal)
    Le,
    /// = (exactly equal)
    Eq,
    /// >= (greater than or equal)
    Ge,
    /// >> (strictly greater than)
    Gt,
}

/// A single parsed dependency entry from a deb822 dependency list.
///
/// Each comma-separated item in fields like `Depends`, `Recommends`, `Provides`,
/// etc. can have:
/// - A package **name**
/// - An optional **architecture** qualifier (e.g. `:amd64`)
/// - An optional version constraint with a [`Relation`] operator and version string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepEntry {
    pub name: String,
    pub arch: Option<String>,
    pub relation: Option<Relation>,
    pub version: Option<String>,
}

// ---------------------------------------------------------------------------
// Winnow parsers
// ---------------------------------------------------------------------------

/// Package name: non-whitespace chars that aren't `:`, `(`, `,`.
fn package_name<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    take_till(1.., |c: char| {
        c == ':' || c == '(' || c == ',' || c.is_whitespace()
    })
    .map(|s: &str| s.trim())
    .parse_next(input)
}

/// Architecture qualifier: `:arch`
fn arch_qualifier<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    preceded(
        ':',
        take_till(1.., |c: char| c == '(' || c == ',' || c.is_whitespace()),
    )
    .parse_next(input)
}

/// Relation operator.
fn relation_op(input: &mut &str) -> ModalResult<Relation> {
    alt((
        literal("<<").value(Relation::Lt),
        literal("<=").value(Relation::Le),
        literal(">>").value(Relation::Gt),
        literal(">=").value(Relation::Ge),
        literal("=").value(Relation::Eq),
    ))
    .parse_next(input)
}

/// Version string: any chars except whitespace and `)`.
fn version_string<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    take_till(1.., |c: char| c.is_whitespace() || c == ')').parse_next(input)
}

/// Version constraint: `(REL version)`
fn version_constraint<'a>(input: &mut &'a str) -> ModalResult<(Relation, &'a str)> {
    delimited(
        preceded(space0, literal("(")),
        preceded(space0, (relation_op, preceded(space1, version_string))),
        preceded(space0, literal(")")),
    )
    .parse_next(input)
}

/// A single dependency entry.
fn dep_entry(input: &mut &str) -> ModalResult<DepEntry> {
    let name = package_name.parse_next(input)?;
    let arch = opt(arch_qualifier).parse_next(input)?;
    let version = opt(version_constraint).parse_next(input)?;

    Ok(DepEntry {
        name: name.to_string(),
        arch: arch.map(|s| s.to_string()),
        relation: version.as_ref().map(|(r, _)| *r),
        version: version.map(|(_, v)| v.to_string()),
    })
}

/// Parse a deb822 dependency list into structured [`DepEntry`] items.
///
/// Many deb822 fields (Depends, Pre-Depends, Recommends, Suggests, Provides,
/// Breaks, Conflicts, Replaces, etc.) use a comma-separated list of package
/// names, each optionally followed by an architecture qualifier and a version
/// constraint in parentheses:
///
/// ```text
/// Depends: libc6 (>= 2.38), ncurses (>= 6.5), zlib1g:amd64
/// ```
pub fn parse_dep_list(value: &str) -> Vec<DepEntry> {
    let mut input = value;
    let entries: Vec<DepEntry> = separated(
        0..,
        preceded(multispace0, dep_entry),
        preceded(multispace0, ','),
    )
    .parse_next(&mut input)
    .unwrap_or_default();
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dep_simple() {
        let result = parse_dep_list("fish, fisher, fisherman");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "fish");
        assert_eq!(result[1].name, "fisher");
        assert_eq!(result[2].name, "fisherman");
        assert!(result[0].relation.is_none());
        assert!(result[0].version.is_none());
        assert!(result[0].arch.is_none());
    }

    #[test]
    fn test_parse_dep_with_version_constraint() {
        let result = parse_dep_list("foo (= 1.0), bar (>= 2.0), baz");
        assert_eq!(result.len(), 3);

        assert_eq!(result[0].name, "foo");
        assert_eq!(result[0].relation, Some(Relation::Eq));
        assert_eq!(result[0].version.as_deref(), Some("1.0"));

        assert_eq!(result[1].name, "bar");
        assert_eq!(result[1].relation, Some(Relation::Ge));
        assert_eq!(result[1].version.as_deref(), Some("2.0"));

        assert_eq!(result[2].name, "baz");
        assert!(result[2].relation.is_none());
        assert!(result[2].version.is_none());
    }

    #[test]
    fn test_parse_dep_empty() {
        let result: Vec<DepEntry> = parse_dep_list("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_dep_with_arch() {
        let result = parse_dep_list("libc6:amd64 (>= 2.38), ncurses");
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].name, "libc6");
        assert_eq!(result[0].arch.as_deref(), Some("amd64"));
        assert_eq!(result[0].relation, Some(Relation::Ge));
        assert_eq!(result[0].version.as_deref(), Some("2.38"));

        assert_eq!(result[1].name, "ncurses");
        assert!(result[1].arch.is_none());
    }

    #[test]
    fn test_parse_dep_all_relations() {
        let result = parse_dep_list("a (<< 1), b (<= 2), c (= 3), d (>= 4), e (>> 5)");
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].relation, Some(Relation::Lt));
        assert_eq!(result[1].relation, Some(Relation::Le));
        assert_eq!(result[2].relation, Some(Relation::Eq));
        assert_eq!(result[3].relation, Some(Relation::Ge));
        assert_eq!(result[4].relation, Some(Relation::Gt));
    }
}
