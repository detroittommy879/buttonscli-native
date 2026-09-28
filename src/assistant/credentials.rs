//! Native-only credential boundary. Do not log underlying keyring errors or values.

use std::collections::BTreeMap;
use std::sync::Mutex;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const SERVICE: &str = "ButtonsCLI Native AI Help";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CredentialError {
    InvalidReference,
    Missing,
    Unavailable,
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidReference => "credential reference is invalid",
            Self::Missing => "no API key is saved for this provider",
            Self::Unavailable => "the operating-system credential store is unavailable",
        };
        f.write_str(message)
    }
}

pub(crate) trait CredentialStore: Send + Sync {
    fn put(&self, reference: &str, value: &str) -> Result<(), CredentialError>;
    fn get(&self, reference: &str) -> Result<Zeroizing<String>, CredentialError>;
    fn delete(&self, reference: &str) -> Result<(), CredentialError>;
}

pub(crate) fn reference(profile: &str, provider_id: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(profile.as_bytes());
    hash.update([0]);
    hash.update(provider_id.as_bytes());
    format!("native:v1:{:x}", hash.finalize())
}

fn valid_reference(reference: &str) -> bool {
    reference
        .strip_prefix("native:v1:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[derive(Default)]
pub(crate) struct SystemCredentialStore;

impl SystemCredentialStore {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    fn entry(reference: &str) -> Result<keyring::Entry, CredentialError> {
        if !valid_reference(reference) {
            return Err(CredentialError::InvalidReference);
        }
        keyring::Entry::new(SERVICE, reference).map_err(|_| CredentialError::Unavailable)
    }
}

impl CredentialStore for SystemCredentialStore {
    fn put(&self, reference: &str, value: &str) -> Result<(), CredentialError> {
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        return Self::entry(reference)?
            .set_password(value)
            .map_err(|_| CredentialError::Unavailable);
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            let _ = (reference, value);
            Err(CredentialError::Unavailable)
        }
    }

    fn get(&self, reference: &str) -> Result<Zeroizing<String>, CredentialError> {
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        return match Self::entry(reference)?.get_password() {
            Ok(value) => Ok(Zeroizing::new(value)),
            Err(keyring::Error::NoEntry) => Err(CredentialError::Missing),
            Err(_) => Err(CredentialError::Unavailable),
        };
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            let _ = reference;
            Err(CredentialError::Unavailable)
        }
    }

    fn delete(&self, reference: &str) -> Result<(), CredentialError> {
        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        return match Self::entry(reference)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Err(CredentialError::Missing),
            Err(_) => Err(CredentialError::Unavailable),
        };
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            let _ = reference;
            Err(CredentialError::Unavailable)
        }
    }
}

#[derive(Default)]
pub(crate) struct SessionCredentialStore {
    values: Mutex<BTreeMap<String, Zeroizing<String>>>,
}

impl CredentialStore for SessionCredentialStore {
    fn put(&self, reference: &str, value: &str) -> Result<(), CredentialError> {
        if !valid_reference(reference) {
            return Err(CredentialError::InvalidReference);
        }
        self.values
            .lock()
            .map_err(|_| CredentialError::Unavailable)?
            .insert(reference.to_owned(), Zeroizing::new(value.to_owned()));
        Ok(())
    }

    fn get(&self, reference: &str) -> Result<Zeroizing<String>, CredentialError> {
        if !valid_reference(reference) {
            return Err(CredentialError::InvalidReference);
        }
        self.values
            .lock()
            .map_err(|_| CredentialError::Unavailable)?
            .get(reference)
            .cloned()
            .ok_or(CredentialError::Missing)
    }

    fn delete(&self, reference: &str) -> Result<(), CredentialError> {
        if !valid_reference(reference) {
            return Err(CredentialError::InvalidReference);
        }
        self.values
            .lock()
            .map_err(|_| CredentialError::Unavailable)?
            .remove(reference)
            .map(|_| ())
            .ok_or(CredentialError::Missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_are_profile_scoped_and_session_store_never_formats_values() {
        let first = reference("profile-one", "provider");
        let second = reference("profile-two", "provider");
        assert_ne!(first, second);
        let store = SessionCredentialStore::default();
        store.put(&first, "FAKE-KEY").unwrap();
        assert_eq!(store.get(&first).unwrap().as_str(), "FAKE-KEY");
        assert_eq!(store.get(&second).err(), Some(CredentialError::Missing));
        assert!(!format!(
            "{first:?} {store_error:?}",
            store_error = CredentialError::Unavailable
        )
        .contains("FAKE-KEY"));
        store.delete(&first).unwrap();
        assert_eq!(store.get(&first).err(), Some(CredentialError::Missing));
    }
}
