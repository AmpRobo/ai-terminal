use std::cell::OnceCell;
use std::collections::BTreeMap;

use ai_terminal_errors::report_error;
use ai_terminal_ui::{AppContext, Entity, ModelContext, SingletonEntity};
use ai_terminal_ui_extras::secure_storage::{self, AppContextExt};
use serde::{Deserialize, Serialize};

use super::SshCredentialKey;

const SECURE_STORAGE_KEY: &str = "SshLoginPasswords";

#[derive(Deserialize)]
struct StoredPassword {
    #[serde(flatten)]
    key: SshCredentialKey,
    password: String,
}

#[derive(Serialize)]
struct StoredPasswordRef<'a> {
    #[serde(flatten)]
    key: &'a SshCredentialKey,
    password: &'a str,
}

/// The SSH passwords saved in secure storage.
#[derive(Default)]
pub struct SavedSshPasswords {
    /// Loaded on first use, so that secure storage is only read once SSH asks for a password.
    loaded: OnceCell<LoadedPasswords>,
}

pub enum SavedSshPasswordsEvent {
    Changed,
}

impl SavedSshPasswords {
    pub fn password(&self, key: &SshCredentialKey, ctx: &AppContext) -> Option<&str> {
        self.loaded(ctx).passwords.get(key).map(String::as_str)
    }

    /// Returns the accounts that have a saved password, in order.
    pub fn accounts(&self, ctx: &AppContext) -> impl Iterator<Item = &SshCredentialKey> {
        self.loaded(ctx).passwords.keys()
    }

    /// Saves `password` for the account, and returns whether the saved password changed.
    pub fn save(
        &mut self,
        key: SshCredentialKey,
        password: String,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        self.update(ctx, |loaded, ctx| loaded.save(key, password, ctx))
    }

    /// Removes the account's saved password, and returns whether there was one.
    pub fn forget(&mut self, key: &SshCredentialKey, ctx: &mut ModelContext<Self>) -> bool {
        self.update(ctx, |loaded, ctx| loaded.forget(key, ctx))
    }

    fn loaded(&self, ctx: &AppContext) -> &LoadedPasswords {
        self.loaded.get_or_init(|| LoadedPasswords::load(ctx))
    }

    fn update(
        &mut self,
        ctx: &mut ModelContext<Self>,
        update: impl FnOnce(&mut LoadedPasswords, &AppContext) -> bool,
    ) -> bool {
        let mut loaded = self
            .loaded
            .take()
            .unwrap_or_else(|| LoadedPasswords::load(ctx));
        let changed = update(&mut loaded, ctx);
        self.loaded = OnceCell::from(loaded);
        if changed {
            ctx.emit(SavedSshPasswordsEvent::Changed);
        }
        changed
    }
}

impl Entity for SavedSshPasswords {
    type Event = SavedSshPasswordsEvent;
}

impl SingletonEntity for SavedSshPasswords {}

struct LoadedPasswords {
    passwords: BTreeMap<SshCredentialKey, String>,
    /// Whether changes may be written, which isn't the case if reading the stored passwords
    /// failed, since writing would erase them.
    writable: bool,
}

impl LoadedPasswords {
    fn load(ctx: &AppContext) -> Self {
        let json = match ctx.secure_storage().read_value(SECURE_STORAGE_KEY) {
            Ok(json) => json,
            Err(secure_storage::Error::NotFound) => String::new(),
            Err(error) => {
                report_error!(
                    anyhow::Error::new(error)
                        .context("Failed to read saved SSH passwords from secure storage")
                );
                return Self {
                    passwords: BTreeMap::new(),
                    writable: false,
                };
            }
        };
        let passwords = if json.is_empty() {
            BTreeMap::new()
        } else {
            match serde_json::from_str::<Vec<StoredPassword>>(&json) {
                Ok(stored) => stored
                    .into_iter()
                    .map(|stored| (stored.key, stored.password))
                    .collect(),
                // The parse error isn't reported since it may quote a password.
                Err(_) => {
                    report_error!("Failed to parse saved SSH passwords");
                    BTreeMap::new()
                }
            }
        };
        Self {
            passwords,
            writable: true,
        }
    }

    fn save(&mut self, key: SshCredentialKey, password: String, ctx: &AppContext) -> bool {
        if !self.writable || self.passwords.get(&key) == Some(&password) {
            return false;
        }
        let previous = self.passwords.insert(key.clone(), password);
        if self.persist(ctx) {
            return true;
        }
        match previous {
            Some(previous) => self.passwords.insert(key, previous),
            None => self.passwords.remove(&key),
        };
        false
    }

    fn forget(&mut self, key: &SshCredentialKey, ctx: &AppContext) -> bool {
        if !self.writable {
            return false;
        }
        let Some(password) = self.passwords.remove(key) else {
            return false;
        };
        if self.persist(ctx) {
            return true;
        }
        self.passwords.insert(key.clone(), password);
        false
    }

    /// Writes the passwords to secure storage, and returns whether that succeeded.
    fn persist(&self, ctx: &AppContext) -> bool {
        let storage = ctx.secure_storage();
        let result = if self.passwords.is_empty() {
            match storage.remove_value(SECURE_STORAGE_KEY) {
                Err(secure_storage::Error::NotFound) => Ok(()),
                result => result,
            }
        } else {
            let stored: Vec<_> = self
                .passwords
                .iter()
                .map(|(key, password)| StoredPasswordRef { key, password })
                .collect();
            match serde_json::to_string(&stored) {
                Ok(json) => storage.write_value_with_owner_only_fallback(SECURE_STORAGE_KEY, &json),
                Err(error) => Err(secure_storage::Error::Unknown(
                    anyhow::Error::new(error).context("Failed to serialize SSH passwords"),
                )),
            }
        };
        match result {
            Ok(()) => true,
            Err(error) => {
                report_error!(
                    anyhow::Error::new(error)
                        .context("Failed to write saved SSH passwords to secure storage")
                );
                false
            }
        }
    }
}

#[cfg(test)]
#[path = "saved_passwords_tests.rs"]
mod tests;
