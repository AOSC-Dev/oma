pub(crate) fn database_filename(url: &str) -> String {
    url.split("://").nth(1).unwrap_or(url).replace('/', "_")
}
