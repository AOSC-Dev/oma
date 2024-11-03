use logind_zbus::{
    manager::{InhibitType, ManagerProxy},
    session::SessionProxy,
};
use tracing::debug;
use zbus::{proxy, zvariant::OwnedFd, Result as zResult};

pub use zbus::Connection;

#[derive(Debug, thiserror::Error)]
pub enum OmaDbusError {
    #[error("Failed to connect system dbus")]
    FailedConnectDbus(zbus::Error),
    #[error("Failed to take wake lock")]
    FailedTakeWakeLock(zbus::Error),
    #[error("Failed to create {0} proxy")]
    FailedCreateProxy(&'static str, zbus::Error),
    #[error("Failed to get battery status")]
    FailedGetBatteryStatus(zbus::Error),
    #[error("Failed to get another oma status")]
    FailedGetOmaStatus(zbus::Error),
    #[error("Failed to get session state")]
    SessionState(zbus::Error),
}

pub type OmaDbusResult<T> = Result<T, OmaDbusError>;

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    /// OnBattery property
    #[zbus(property)]
    fn on_battery(&self) -> zResult<bool>;
}

#[proxy(
    interface = "io.aosc.Oma1",
    default_service = "io.aosc.Oma",
    default_path = "/io/aosc/Oma"
)]
trait OmaDbus {
    async fn get_status(&self) -> zResult<String>;
}

pub async fn create_dbus_connection() -> OmaDbusResult<Connection> {
    Connection::system()
        .await
        .map_err(OmaDbusError::FailedConnectDbus)
}

pub async fn get_another_oma_status(conn: &Connection) -> OmaDbusResult<String> {
    let proxy = OmaDbusProxy::new(conn)
        .await
        .map_err(|e| OmaDbusError::FailedCreateProxy("oma1", e))?;

    let s = proxy
        .get_status()
        .await
        .map_err(OmaDbusError::FailedGetOmaStatus)?;

    Ok(s)
}

/// Take the wake lock and prevent the system from sleeping. Drop the returned file handle to release the lock.
pub async fn take_wake_lock(
    conn: &Connection,
    why: &str,
    binary_name: &str,
) -> OmaDbusResult<Vec<OwnedFd>> {
    let proxy = ManagerProxy::new(conn)
        .await
        .map_err(|e| OmaDbusError::FailedCreateProxy("login1", e))?;

    let mut fds = Vec::new();
    for what in [InhibitType::Shutdown, InhibitType::Sleep] {
        let fd = proxy
            .inhibit(what, binary_name, why, "block")
            .await
            .map_err(OmaDbusError::FailedTakeWakeLock)?;

        fds.push(fd);
    }

    debug!("take wake lock: {:?}", fds);

    Ok(fds)
}

/// Get session name
pub async fn session_name(conn: &Connection) -> OmaDbusResult<String> {
    let session = SessionProxy::builder(conn)
        .path("/org/freedesktop/login1/session/auto")
        .map_err(|e| OmaDbusError::FailedCreateProxy("login1", e))?
        .build()
        .await
        .map_err(|e| OmaDbusError::FailedCreateProxy("login1", e))?;

    let state = session
        .service()
        .await
        .map_err(OmaDbusError::SessionState)?;

    Ok(state)
}

/// Check computer is using battery (like laptop)
pub async fn is_using_battery(conn: &Connection) -> OmaDbusResult<bool> {
    let proxy = UPowerProxy::new(conn)
        .await
        .map_err(|e| OmaDbusError::FailedCreateProxy("upower", e))?;

    proxy
        .on_battery()
        .await
        .map_err(OmaDbusError::FailedGetBatteryStatus)
}
