use zbus::{dbus_proxy, Result as zResult};

pub use zbus::Connection;
pub use zbus::Error as zError;

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
    fn inhibit(
        &self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> zResult<zbus::zvariant::OwnedFd>;
}

pub async fn create_dbus_connection() -> zResult<Connection> {
    Connection::system().await
}

/// Take the wake lock and prevent the system from sleeping. Drop the returned file handle to release the lock.
pub async fn take_wake_lock(
    conn: &Connection,
    why: &str,
    binary_name: &str,
) -> zResult<zbus::zvariant::OwnedFd> {
    let proxy = Login1Proxy::new(conn).await?;

    proxy
        .inhibit("shutdown:sleep", binary_name, why, "block")
        .await
}

/// Check computer is using battery (like laptop)
pub async fn is_using_battery(conn: &Connection) -> zResult<bool> {
    let proxy = UPowerProxy::new(conn).await?;

    proxy.on_battery().await
}
