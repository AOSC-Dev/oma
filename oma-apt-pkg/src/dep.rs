//! Parsing of deb822 dependency list fields (Depends, Recommends, Provides, etc.).

use std::str::FromStr;

/// The version constraint operator in a dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// << (strictly less than)
    StrictLt,
    /// <= (less than or equal)
    Lt,
    /// = (exactly equal)
    Eq,
    /// >= (greater than or equal)
    Ge,
    /// >> (strictly greater than)
    StrictGt,
}

impl FromStr for Relation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "<<" => Ok(Self::StrictLt),
            "<=" => Ok(Self::Lt),
            "=" => Ok(Self::Eq),
            ">=" => Ok(Self::Ge),
            ">>" => Ok(Self::StrictGt),
            _ => Err(()),
        }
    }
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
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_dep_entry)
        .collect()
}

fn parse_dep_entry(s: &str) -> DepEntry {
    // Split off the version constraint in parentheses
    let (rest, version_part) = if let Some(idx) = s.find('(') {
        let rest = s[..idx].trim();
        let version_part = s[idx + 1..].trim_end();
        let version_part = version_part.strip_suffix(')').unwrap_or(version_part).trim();
        (rest, Some(version_part))
    } else {
        (s.trim(), None)
    };

    // Split off the architecture qualifier
    let (name, arch) = if let Some(idx) = rest.find(':') {
        let name = rest[..idx].trim().to_string();
        let arch = rest[idx + 1..].trim().to_string();
        (name, Some(arch))
    } else {
        (rest.trim().to_string(), None)
    };

    // Parse the version constraint
    let (relation, version) = match version_part {
        Some(vp) => {
            let parts: Vec<&str> = vp.splitn(2, |c: char| c.is_whitespace()).collect();
            if parts.len() == 2 {
                let rel = Relation::from_str(parts[0].trim()).ok();
                let ver = parts[1].trim().to_string();
                (rel, Some(ver))
            } else {
                (None, Some(vp.to_string()))
            }
        }
        None => (None, None),
    };

    DepEntry {
        name,
        arch,
        relation,
        version,
    }
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
        assert_eq!(result[0].relation, Some(Relation::StrictLt));
        assert_eq!(result[1].relation, Some(Relation::Lt));
        assert_eq!(result[2].relation, Some(Relation::Eq));
        assert_eq!(result[3].relation, Some(Relation::Ge));
        assert_eq!(result[4].relation, Some(Relation::StrictGt));
    }
}
