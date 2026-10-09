//! Explicit trusted-controller tenant mapping port, not a production authenticator.
use guardengine::integration::eligibility::{AuthorityError, Validity};
use std::collections::BTreeSet;
#[derive(Clone)]
pub struct CallerRecord {
    pub tenant: String,
    pub principal: String,
    pub validity: Validity,
}
/// Authenticate/refresh outside candidate code. The trait is not a security sandbox.
pub trait CallerPort {
    fn resolve(&self, reference: &str) -> Result<CallerRecord, AuthorityError>;
}
#[derive(Clone, Copy)]
pub struct Access<'a> {
    pub port: &'a dyn CallerPort,
    pub reference: &'a str,
    pub now: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustError {
    InvalidPolicy,
}
pub struct TenantPolicy {
    pub(crate) tenant: String,
    pub(crate) repo: String,
    consumers: BTreeSet<String>,
    readers: BTreeSet<String>,
}
impl TenantPolicy {
    pub fn freeze(
        tenant: &str,
        repo: &str,
        consumers: BTreeSet<String>,
        readers: BTreeSet<String>,
    ) -> Result<Self, TrustError> {
        if consumers.is_empty()
            || readers.is_empty()
            || consumers.len() > 64
            || readers.len() > 64
            || std::iter::once(tenant)
                .chain(std::iter::once(repo))
                .chain(consumers.iter().map(String::as_str))
                .chain(readers.iter().map(String::as_str))
                .any(|v| v.len() > 256 || v.trim().is_empty() || v.chars().any(char::is_control))
        {
            return Err(TrustError::InvalidPolicy);
        }
        Ok(Self {
            tenant: tenant.into(),
            repo: repo.into(),
            consumers,
            readers,
        })
    }
    pub(crate) fn authorize(&self, access: Access<'_>, read: bool) -> Result<String, ()> {
        if access.reference.is_empty() || access.reference.len() > 256 || access.now < 0 {
            return Err(());
        }
        let caller = access.port.resolve(access.reference).map_err(|_| ())?;
        if caller.tenant != self.tenant
            || !(if read { &self.readers } else { &self.consumers }).contains(&caller.principal)
            || caller.validity.revoked
            || caller.validity.issued_at > access.now
            || caller.validity.issued_at < 0
            || caller.validity.expires_at <= access.now
        {
            return Err(());
        }
        Ok(crate::digest(caller.principal.as_bytes()))
    }
}
