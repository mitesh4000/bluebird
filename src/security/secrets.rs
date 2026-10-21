//! OS keychain-backed secret storage via `keyring`.

use anyhow::{Context, Result};
use uuid::Uuid;

const SERVICE: &str = "bluebird-api-client";

/// Store a secret; returns a reference id to persist in SQLite.
pub fn store_secret(label: &str, secret: &str) -> Result<String> {
    let id = format!("bluebird/{}", Uuid::new_v4());
    let entry = keyring::Entry::new(SERVICE, &id).context("keyring entry")?;
    entry
        .set_password(secret)
        .with_context(|| format!("store secret for {}", label))?;
    Ok(id)
}

/// Load a secret by reference id.
pub fn load_secret(id: &str) -> Result<String> {
    let entry = keyring::Entry::new(SERVICE, id).context("keyring entry")?;
    entry.get_password().context("load secret")
}

/// Delete a secret by reference id.
pub fn delete_secret(id: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, id).context("keyring entry")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
