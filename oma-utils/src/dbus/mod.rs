use std::fmt::Display;

use logind_zbus::session::SessionProxy;
use tracing::debug;
use zbus::{Result as zResult, proxy, zvariant::OwnedFd};

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

    debug!("{}", s);

    Ok(s)
}

#[proxy(
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

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum InhibitType {
    Shutdown,
    Sleep,
    Idle,
    HandlePowerKey,
    HandleSuspendKey,
    HandleHibernateKey,
    HandleLidSwitch,
}

impl Display for InhibitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InhibitType::Shutdown => "shutdown",
                InhibitType::Sleep => "sleep",
                InhibitType::Idle => "idle",
                InhibitType::HandlePowerKey => "handle-power-key",
                InhibitType::HandleSuspendKey => "handle-suspend-key",
                InhibitType::HandleHibernateKey => "handle-hibernate-key",
                InhibitType::HandleLidSwitch => "handle-lid-switch",
            }
        )
    }
}

pub struct InhibitTypeUnion<'a>(pub &'a [InhibitType]);

impl InhibitTypeUnion<'_> {
    pub const fn all() -> Self {
        Self(&[
            InhibitType::Sleep,
            InhibitType::Shutdown,
            InhibitType::Idle,
            InhibitType::HandleSuspendKey,
            InhibitType::HandlePowerKey,
            InhibitType::HandleLidSwitch,
            InhibitType::HandleHibernateKey,
        ])
    }
}

impl Display for InhibitTypeUnion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(":")
        )
    }
}

/// Take the wake lock and prevent the system from sleeping. Drop the returned file handle to release the lock.
pub async fn take_wake_lock(
    conn: &Connection,
    what: InhibitTypeUnion<'_>,
    why: &str,
    binary_name: &str,
) -> OmaDbusResult<OwnedFd> {
    let proxy = Login1Proxy::new(conn)
        .await
        .map_err(|e| OmaDbusError::FailedCreateProxy("login1", e))?;

    let fd = proxy
        .inhibit(&what.to_string(), binary_name, why, "block")
        .await
        .map_err(OmaDbusError::FailedTakeWakeLock)?;

    debug!("take wake lock: {:?}", fd);

    Ok(fd)
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
