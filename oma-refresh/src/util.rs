pub(crate) fn database_filename(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .replace('/', "_")
        .replace('+', "%252b")
}

#[test]
fn test_database_filename() {
    // Mirror name contains '+' must be url encode twice
    let s = "https://repo.aosc.io/debs/dists/x264-0+git20240305/InRelease";
    let res = database_filename(s);

    assert_eq!(
        res,
        "repo.aosc.io_debs_dists_x264-0%252bgit20240305_InRelease"
    )
}
