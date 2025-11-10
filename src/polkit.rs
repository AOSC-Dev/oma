use anyhow::Context;
use zbus::fdo;
use zbus_polkit::policykit1::CheckAuthorizationFlags;

pub async fn get_permission_use_polkit() -> fdo::Result<()> {
    let connection = zbus::Connection::system().await?;
    let polkit = zbus_polkit::policykit1::AuthorityProxy::new(&connection)
        .await
        .context("could not connect to polkit authority daemon")
        .map_err(|e| fdo::Error::Failed(e.to_string()))?;

    let pid = std::process::id();

    let permitted = if pid == 0 {
        true
    } else {
        let subject = zbus_polkit::policykit1::Subject::new_for_owner(pid, None, None)
            .context("could not create policykit1 subject")
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        let b = polkit
            .check_authorization(
                &subject,
                "io.aosc.oma.apply.run",
                &std::collections::HashMap::new(),
                CheckAuthorizationFlags::AllowUserInteraction.into(),
                "",
            )
            .await
            .context("could not check policykit authorization")
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;

        b.is_authorized
    };

    if permitted {
        Ok(())
    } else {
        Err(fdo::Error::Failed(
            "Operation not permitted by Polkit".to_string(),
        ))
    }
}
