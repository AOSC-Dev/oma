use rusqlite::Connection;
pub use rusqlite::Error;
use std::path::Path;

pub struct UbuntuCmdNotFound {
    db: Connection,
}

impl UbuntuCmdNotFound {
    const DEFAULT_DB_PATH: &str = "/var/lib/command-not-found/commands.db";
    pub fn new(db: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let db = Connection::open(db)?;

        Ok(Self { db })
    }

    pub fn default_new() -> Result<Self, rusqlite::Error> {
        Self::new(Self::DEFAULT_DB_PATH)
    }

    pub fn query_where_command_like(&self, query: &str) -> Result<Vec<(String, String)>, Error> {
        let mut stmt = self
            .db
            .prepare("SELECT pkgID, command FROM commands WHERE command LIKE ?1")?;
        let query_str = format!("{}%", query);
        let res_iter = stmt.query_map([query_str], |row| {
            let pkg_id: i64 = row.get(0)?;
            let cmd: String = row.get(1)?;

            Ok((pkg_id, cmd))
        })?;
        let mut res = vec![];
        for i in res_iter {
            let (pkg_id, cmd) = i?;
            let pkgs = self.get_pkg_from_from_pkg_id(pkg_id)?;

            for pkg in pkgs {
                res.push((pkg, cmd.clone()));
            }
        }
        Ok(res)
    }

    pub fn query_where_command_count(&self, query: &str) -> Result<i64, Error> {
        let mut stmt = self
            .db
            .prepare("SELECT COUNT(command) AS count FROM commands WHERE command = ?1")?;
        let mut res_iter = stmt.query_map([query], |row| row.get(0))?;

        if let Some(Ok(n)) = res_iter.next() {
            return Ok(n);
        }

        Ok(0)
    }

    pub fn query_where_command_like_count(&self, query: &str) -> Result<i64, Error> {
        let mut stmt = self
            .db
            .prepare("SELECT COUNT(command) AS count FROM commands WHERE command LIKE ?")?;
        let query_str = format!("{}%", query);
        let mut res_iter = stmt.query_map([query_str], |row| row.get(0))?;

        if let Some(Ok(n)) = res_iter.next() {
            return Ok(n);
        }

        Ok(0)
    }

    pub fn query_where_command(&self, query: &str) -> Result<Vec<(String, String)>, Error> {
        let mut stmt = self
            .db
            .prepare("SELECT pkgID, command FROM commands WHERE command = ?1")?;
        let res_iter = stmt.query_map([query], |row| {
            let pkg_id: i64 = row.get(0)?;
            let cmd: String = row.get(1)?;

            Ok((pkg_id, cmd))
        })?;
        let mut res = vec![];
        for i in res_iter {
            let (pkg_id, cmd) = i?;
            let pkgs = self.get_pkg_from_from_pkg_id(pkg_id)?;

            for pkg in pkgs {
                res.push((pkg, cmd.clone()));
            }
        }
        Ok(res)
    }

    fn get_pkg_from_from_pkg_id(&self, pkg_id: i64) -> Result<Vec<String>, rusqlite::Error> {
        let mut res = vec![];
        let mut stmt = self
            .db
            .prepare("SELECT name FROM packages where pkgID = ?1")?;
        let pkgs = stmt.query_map([pkg_id], |row| row.get(0))?;

        for pkg in pkgs {
            let pkg = pkg?;
            res.push(pkg);
        }

        Ok(res)
    }
}
