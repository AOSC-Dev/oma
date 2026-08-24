//! Parsing of the date/time strings found in Debian `Release`/`InRelease`
//! files (the `Date:` and `Valid-Until:` fields).
//!
//! The parser mirrors apt's `RFC1123StrToTime`
//! (apt-pkg/contrib/strutl.cc on master, lines ~1038-1132), which accepts the
//! RFC 1123, RFC 850 and asctime layouts. jiff does the actual calendar, clock
//! and offset validation.

use jiff::Timestamp;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ParseDateError {
    #[error(transparent)]
    ParseError(#[from] jiff::Error),
    // Raised for structural problems across all three accepted layouts, e.g.
    // a missing weekday, a localized weekday, or a wrongly placed comma.
    #[error("Invalid date: expected RFC 1123, RFC 850 or asctime format")]
    BadDate,
    #[error("Unknown timezone `{0}`")]
    BadZone(String),
}

pub(crate) fn parse_date(date: &str) -> Result<Timestamp, ParseDateError> {
    // Mirror of apt's `RFC1123StrToTime` (apt-pkg/contrib/strutl.cc on master,
    // lines ~1038-1132), which accepts RFC 1123, RFC 850 and asctime forms.
    // Weekday validation and layout selection mirror lines ~1048-1097,
    // timezone handling lines ~1097-1127, and the final strict parse lines
    // ~1127-1132. The only deliberate divergence is accepting non-UTC numeric
    // offsets (see parse_zone) instead of rejecting them. jiff does the actual
    // calendar, clock and offset validation.
    //
    // Collapse any whitespace so fields are predictable, mirroring apt's
    // `operator>>` which skips arbitrary whitespace between fields.
    let date = date.split_ascii_whitespace().collect::<Vec<_>>().join(" ");

    let (weekday, rest) = date.split_once(' ').ok_or(ParseDateError::BadDate)?;

    // Like apt (strutl.cc lines ~1052-1059), only the first three letters are
    // checked to reject localized weekdays; the *length* of the token then
    // selects the date layout (strutl.cc lines ~1060-1097). This is as lenient
    // as apt: a typo like `Thursdayyyyy,` still routes to RFC 850.
    let wd = weekday
        .get(..weekday.len().min(3))
        .ok_or(ParseDateError::BadDate)?;
    if !matches!(
        wd.to_ascii_lowercase().as_str(),
        "sun" | "mon" | "tue" | "wed" | "thu" | "fri" | "sat"
    ) {
        return Err(ParseDateError::BadDate);
    }

    match weekday.len() {
        // ANSI C asctime(): "Sun Nov  6 08:49:37 1994" — no timezone, UTC.
        3 => {
            let datetime = jiff::civil::DateTime::strptime("%b %e %H:%M:%S %Y", rest)?;
            Ok(jiff::tz::Offset::UTC.to_timestamp(datetime)?)
        }
        // RFC 1123: "Sun, 06 Nov 1994 08:49:37 GMT"
        4 if weekday.ends_with(',') => parse_comma_date(rest, "%d %b %Y %H:%M:%S"),
        // RFC 850: "Sunday, 06-Nov-94 08:49:37 GMT"
        //
        // Two-digit year: jiff's `%y` follows the POSIX pivot (69-99 →
        // 1969-1999, 00-68 → 2000-2068), while apt computes `1900 + yy`
        // (strutl.cc line ~1086). They agree for 69-99 and diverge for 00-68,
        // where we keep jiff's rule, which maps modern dates (e.g. "24" →
        // 2024) correctly.
        _ if weekday.ends_with(',') => parse_comma_date(rest, "%d-%b-%y %H:%M:%S"),
        _ => Err(ParseDateError::BadDate),
    }
}

/// Parse the fields after `Weekday,` — date + time + trailing timezone token —
/// for the RFC 1123 (`dd Mon yyyy`) and RFC 850 (`dd-Mon-yy`) layouts.
///
/// Mirrors apt's RFC1123StrToTime (strutl.cc lines ~1062-1097): the date/time
/// fields and the trailing zone token are read separately, then the datetime
/// is parsed strictly (equivalent to apt's `strptime("%Y-%m-%d %H:%M:%S")` at
/// lines ~1127-1132).
fn parse_comma_date(rest: &str, format: &str) -> Result<Timestamp, ParseDateError> {
    let (date_part, zone) = rest
        .rsplit_once(' ')
        .ok_or_else(|| ParseDateError::BadZone("missing timezone".to_string()))?;
    let offset = parse_zone(zone)?;
    let datetime = jiff::civil::DateTime::strptime(format, date_part)?;

    Ok(offset.to_timestamp(datetime)?)
}

/// Resolve the trailing timezone token of an RFC 1123/850 date to a fixed UTC
/// offset.
///
/// Mirrors apt's RFC1123StrToTime (strutl.cc lines ~1097-1113), which resolves
/// the zone by hand (strptime's `%Z` is a no-op on glibc). With a case
/// sensitive comparison apt accepts exactly:
///
///   - the named zones `GMT`, `UTC` and `Z` (strutl.cc line ~1099), or
///   - any zone whose integer value is 0 and is fully consumed, i.e. any
///     all-zero token with or without a sign — `+0000`, `-0000`, `0000`,
///     `+0`, `-0`, `+000`, `0`, ... (strutl.cc lines ~1100-1111;
///     see also the note in apt-pkg/contrib/strutl.h lines ~61-79).
///
/// We deliberately extend this (as oma's previous rfc2822-based parser did):
///
///   - `UT` (named by RFC 1123) and lowercase `z` are treated as UTC;
///   - any other numeric offset (`+HH`, `+HHMM`, `+HH:MM`, including non-zero
///     ones such as `+0530`/`-0500`) is accepted and converted correctly,
///     where apt would reject the whole date.
fn parse_zone(token: &str) -> Result<jiff::tz::Offset, ParseDateError> {
    match token {
        "UTC" | "GMT" | "UT" | "Z" | "z" => return Ok(jiff::tz::Offset::UTC),
        _ => {}
    }

    let (sign, digits) = match token.as_bytes().split_first() {
        Some((b'+', rest)) => (1i32, rest),
        Some((b'-', rest)) => (-1i32, rest),
        // Like apt (`stoi(zone) == 0`), a sign-less all-zero zone such as
        // `0000` or `0` is also accepted; anything else is rejected.
        _ => {
            let z = token
                .parse::<i32>()
                .map_err(|_| ParseDateError::BadZone(token.to_string()))?;
            return if z == 0 {
                Ok(jiff::tz::Offset::UTC)
            } else {
                Err(ParseDateError::BadZone(token.to_string()))
            };
        }
    };

    // Like apt (`stoi(zone) == 0` with full consumption), any all-zero token
    // — signed or not (`+0`, `-0`, `+000`, `+000000`, ...) — is UTC, even
    // though it does not match the `HH`/`HHMM`/`HH:MM` shapes below. The sign
    // is discarded, so this is always a zero offset. A lone sign (`+`/`-`)
    // has no digits to consume and falls through to the shapes to be
    // rejected, like apt's `stoi("+")` which throws.
    if !digits.is_empty() && digits.iter().all(|b| *b == b'0') {
        return Ok(jiff::tz::Offset::UTC);
    }

    // Accept `HH`, `HHMM` and `HH:MM`.
    let (hh, mm) = match digits {
        [a, b] => (pair_to_num(*a, *b, token)?, 0),
        [a, b, c, d] => (pair_to_num(*a, *b, token)?, pair_to_num(*c, *d, token)?),
        [a, b, b':', c, d] => (pair_to_num(*a, *b, token)?, pair_to_num(*c, *d, token)?),
        _ => return Err(ParseDateError::BadZone(token.to_string())),
    };

    if hh > 23 || mm > 59 {
        return Err(ParseDateError::BadZone(token.to_string()));
    }

    jiff::tz::Offset::from_seconds(sign * (hh * 3600 + mm * 60)).map_err(ParseDateError::from)
}

fn pair_to_num(a: u8, b: u8, token: &str) -> Result<i32, ParseDateError> {
    if !a.is_ascii_digit() || !b.is_ascii_digit() {
        return Err(ParseDateError::BadZone(token.to_string()));
    }
    Ok(((a - b'0') * 10 + (b - b'0')) as i32)
}

#[test]
fn test_apt_date_parser() {
    // These are the three date layouts handled by apt's RFC1123StrToTime.
    let cases = [
        ("Thu, 02 May 2024 09:58:03 +0000", "2024-05-02T09:58:03Z"),
        ("Thursday, 02-May-24 09:58:03 +0000", "2024-05-02T09:58:03Z"),
        ("Thu May 2 09:58:03 2024", "2024-05-02T09:58:03Z"),
        ("Thu, 02 May 2024 09:58:03 +0530", "2024-05-02T04:28:03Z"),
    ];
    for (input, expected) in cases {
        assert_eq!(parse_date(input).unwrap().to_string(), expected);
    }

    // Apt's parser is permissive about whitespace between fields.
    assert!(parse_date("Thu, 02 May 2024  09:58:03 +0000").is_ok());

    // RFC 1123 specifies GMT as the zone; apt additionally accepts UTC (a
    // compatibility extension, and what Debian Release files actually emit)
    // and Z. We also accept `UT` and lowercase `z`. Single-digit hours are
    // tolerated like apt.
    assert_eq!(
        parse_date("Thu, 02 May 2024 09:58:03 UTC")
            .unwrap()
            .to_string(),
        "2024-05-02T09:58:03Z"
    );
    assert_eq!(
        parse_date("Thu, 02 May 2024 09:58:03 GMT")
            .unwrap()
            .to_string(),
        "2024-05-02T09:58:03Z"
    );
    assert_eq!(
        parse_date("Thu, 02 May 2024  9:58:03 UTC")
            .unwrap()
            .to_string(),
        "2024-05-02T09:58:03Z"
    );
    assert_eq!(
        parse_date("Thursday, 02-May-24 09:58:03 UTC")
            .unwrap()
            .to_string(),
        "2024-05-02T09:58:03Z"
    );

    // Negative UTC offsets must not be mistaken for an RFC 850 date.
    assert_eq!(
        parse_date("Thu, 02 May 2024 09:58:03 -0500")
            .unwrap()
            .to_string(),
        "2024-05-02T14:58:03Z"
    );
    assert_eq!(
        parse_date("Thursday, 02-May-24 09:58:03 -0500")
            .unwrap()
            .to_string(),
        "2024-05-02T14:58:03Z"
    );

    // Do not silently accept an unknown date layout or an unknown timezone.
    assert!(parse_date("not a date").is_err());
    assert!(parse_date("Thu, 02 May 2024 09:58:03 PST").is_err());
}

#[test]
fn test_whitespace_tolerance() {
    // Like apt's `operator>>`, any ASCII whitespace may separate fields.
    let cases = [
        "Sun,\t06 Nov 1994 08:49:37 UTC",
        "  Sun, 06 Nov 1994 08:49:37 UTC",
        "Sun, 06 Nov 1994 08:49:37\tUTC",
        "Sun, 06 Nov 1994 08:49:37 UTC  ",
        "Sun\tNov\t6\t08:49:37\t1994",
    ];
    for input in cases {
        assert_eq!(
            parse_date(input).unwrap().to_string(),
            "1994-11-06T08:49:37Z",
            "input: {input:?}"
        );
    }
}

#[test]
fn test_zone_and_year_edges() {
    // Timezone tokens. apt's accepted set (case-sensitive) is exactly
    // {GMT, UTC, Z} plus anything `stoi`-ing to 0 (`+0000`, `-0000`, `0000`,
    // `0`, `+0`, `+000`, ...). We additionally accept `UT`, lowercase `z`
    // and non-zero offsets (converted correctly) — see parse_zone.
    let accepted = [
        ("Sun, 06 Nov 1994 08:49:37 GMT", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 UTC", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 Z", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 z", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 UT", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 +0000", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 -0000", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 0000", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 0", "1994-11-06T08:49:37Z"),
        // Signed all-zero tokens: apt accepts any fully-consumed integer
        // zero regardless of shape (`stoi("+0") == 0`), so must we.
        ("Sun, 06 Nov 1994 08:49:37 +0", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 -0", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 +000", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 -000", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 +000000", "1994-11-06T08:49:37Z"),
        // Deliberate extensions over apt: non-zero offsets are converted.
        ("Sun, 06 Nov 1994 08:49:37 +0530", "1994-11-06T03:19:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 -0500", "1994-11-06T13:49:37Z"),
    ];
    for (input, expected) in accepted {
        assert_eq!(
            parse_date(input).unwrap().to_string(),
            expected,
            "input: {input}"
        );
    }

    // Zone comparison is case-sensitive like apt; non-zero without a sign is
    // rejected (apt: `stoi("0530") == 530`).
    let rejected = [
        "Sun, 06 Nov 1994 08:49:37 gmt",
        "Sun, 06 Nov 1994 08:49:37 utc",
        "Sun, 06 Nov 1994 08:49:37 ut",
        "Sun, 06 Nov 1994 08:49:37 PST",
        "Sun, 06 Nov 1994 08:49:37 0530",
        // A lone sign has no digits to consume; apt's `stoi("+")` throws.
        "Sun, 06 Nov 1994 08:49:37 +",
        "Sun, 06 Nov 1994 08:49:37 -",
    ];
    for input in rejected {
        assert!(parse_date(input).is_err(), "should reject: {input}");
    }

    // RFC 850 two-digit years: jiff follows the POSIX pivot (69-99 → 19xx,
    // 00-68 → 20xx); apt computes `1900 + yy` (they agree for 69-99, diverge
    // for 00-68 — we keep jiff's rule).
    let years = [
        ("Sunday, 06-Nov-68 08:49:37 GMT", "2068-11-06T08:49:37Z"),
        ("Sunday, 06-Nov-69 08:49:37 GMT", "1969-11-06T08:49:37Z"),
        ("Sunday, 06-Nov-70 08:49:37 GMT", "1970-11-06T08:49:37Z"),
        ("Sunday, 06-Nov-94 08:49:37 GMT", "1994-11-06T08:49:37Z"),
        ("Sunday, 06-Nov-99 08:49:37 GMT", "1999-11-06T08:49:37Z"),
    ];
    for (input, expected) in years {
        assert_eq!(
            parse_date(input).unwrap().to_string(),
            expected,
            "input: {input}"
        );
    }

    // Like apt, only the first three letters of the weekday are checked, so a
    // long weekday still routes to RFC 850.
    assert_eq!(
        parse_date("Thursdayyyyy, 02-May-24 09:58:03 +0000")
            .unwrap()
            .to_string(),
        "2024-05-02T09:58:03Z"
    );
}

#[test]
fn test_apt_suite_cases() {
    // Accepted by apt's own test suite (test/libapt/strutil_test.cc).
    let cases = [
        ("Sun, 06 Nov 1994 08:49:37 GMT", "1994-11-06T08:49:37Z"),
        ("Sun, 6 Nov 1994 08:49:37 UTC", "1994-11-06T08:49:37Z"),
        ("Sun,  6 Nov 1994 08:49:37 UTC", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994  8:49:37 UTC", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 -0000", "1994-11-06T08:49:37Z"),
        ("Sun, 06 Nov 1994 08:49:37 +0000", "1994-11-06T08:49:37Z"),
        ("Sunday, 06-Nov-94 08:49:37 GMT", "1994-11-06T08:49:37Z"),
        ("Sunday,  6-Nov-94 08:49:37 GMT", "1994-11-06T08:49:37Z"),
        ("Sunday, 06-Nov-94 8:49:37 GMT", "1994-11-06T08:49:37Z"),
        ("Sun Nov  6 08:49:37 1994", "1994-11-06T08:49:37Z"),
        ("Sun Nov 06 08:49:37 1994", "1994-11-06T08:49:37Z"),
        ("Sun Nov  6  8:49:37 1994", "1994-11-06T08:49:37Z"),
    ];
    for (input, expected) in cases {
        assert_eq!(parse_date(input).unwrap().to_string(), expected);
    }

    // Rejected by apt's own test suite.
    let rejected = [
        "So, 06 Nov 1994 08:49:37 UTC",
        ", 06 Nov 1994 08:49:37 UTC",
        "Son, 06 Nov 1994 08:49:37 UTC",
        "Sun: 06 Nov 1994 08:49:37 UTC",
        "Sun, 06 Nov 1994 08:49:37",
        "Sun, 06 Nov 1994 08:49:37 GMT+1",
        "Sunday, 06 Nov 1994 GMT",
        "Sunday, 06 Nov 1994 08:49:37 GMT",
        "Sun, 06-Nov-94 08:49:37 GMT",
        "Sonntag, 06 Nov 1994 08:49:37 GMT",
        "domingo Nov 6 08:49:37 1994",
        "Sunday: 06-Nov-94 08:49:37 GMT",
        "Sunday, 06-Nov-94 08:49:37 GMT+1",
    ];
    for input in rejected {
        assert!(parse_date(input).is_err(), "should reject: {input}");
    }
}
