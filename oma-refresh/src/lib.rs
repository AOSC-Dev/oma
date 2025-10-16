mod config;
pub mod db;
pub mod inrelease;
mod sourceslist;
mod util;

#[cfg(test)]
mod test {
    use std::sync::{LazyLock, Mutex};
    pub(crate) static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
}
