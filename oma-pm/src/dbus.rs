use spdlog::debug;
use zbus::{Connection, Result, interface, proxy};

pub struct OmaBus {
    pub status: Status,
}

pub enum Status {
    Pending,
    Downloading,
    Working(String),
}

#[interface(name = "io.aosc.Oma1")]
impl OmaBus {
    fn get_status(&self) -> String {
        match &self.status {
            Status::Pending => "Pending".to_string(),
            Status::Downloading => "Downloading".to_string(),
            Status::Working(pkg) => pkg.to_string(),
        }
    }

    fn change_status(&mut self, status: &str) {
        match status {
            "pending" => self.status = Status::Pending,
            "Downloading" => self.status = Status::Downloading,
            status if status.starts_with("i ") => {
                let pkg = status.strip_prefix("i ").unwrap();
                self.status = Status::Working(pkg.to_string());
            }
            _ => (),
        }
    }
}

#[proxy(
    interface = "io.aosc.Oma1",
    default_service = "io.aosc.Oma",
    default_path = "/io/aosc/Oma"
)]
trait OmaDbus {
    async fn change_status(&self, name: &str) -> Result<()>;
}

pub async fn change_status(connection: &Connection, status: &str) -> Result<()> {
    let proxy = OmaDbusProxy::new(connection).await?;
    proxy.change_status(status).await?;

    Ok(())
}

pub(crate) async fn create_session() -> Result<Connection> {
    let conn = zbus::connection::Builder::system()?
        .name("io.aosc.Oma")?
        .serve_at(
            "/io/aosc/Oma",
            OmaBus {
                status: Status::Pending,
            },
        )?
        .build()
        .await?;

    debug!("zbus session created");

    Ok(conn)
}
