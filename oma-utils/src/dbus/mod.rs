use zbus::{dbus_proxy, zvariant::OwnedFd, Result as zResult};

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
}

pub type OmaDbusResult<T> = Result<T, OmaDbusError>;

#[dbus_proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    /// OnBattery property
    #[dbus_proxy(property)]
    fn on_battery(&self) -> zResult<bool>;
}

#[dbus_proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1 {
    /// Inhibit method
    fn inhibit(&self, what: &str, who: &str, why: &str, mode: &str) -> zResult<OwnedFd>;
}

pub async fn create_dbus_connection() -> OmaDbusResult<Connection> {
    Connection::system()
        .await
        .map_err(OmaDbusError::FailedConnectDbus)
}

/// Take the wake lock and prevent the system from sleeping. Drop the returned file handle to release the lock.
pub async fn take_wake_lock(
    conn: &Connection,
    why: &str,
    binary_name: &str,
) -> OmaDbusResult<OwnedFd> {
    let proxy = Login1Proxy::new(conn)
        .await
        .map_err(|e| OmaDbusError::FailedCreateProxy("login1", e))?;

    proxy
        .inhibit("shutdown:sleep", binary_name, why, "block")
        .await
        .map_err(OmaDbusError::FailedTakeWakeLock)
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
