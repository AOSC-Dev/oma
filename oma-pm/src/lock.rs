use oma_apt::{raw::apt_unlock, util::apt_lock};

use crate::apt::{OmaAptError, OmaAptResult};

pub struct AptLockGuard;

impl AptLockGuard {
    pub fn acquire() -> OmaAptResult<Self> {
        apt_lock().map_err(OmaAptError::LockApt)?;
        Ok(Self)
    }
}

impl Drop for AptLockGuard {
    fn drop(&mut self) {
        apt_unlock();
    }
}
