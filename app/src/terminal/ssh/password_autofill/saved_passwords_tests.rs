use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ai_terminal_ui::{App, ModelHandle};
use ai_terminal_ui_extras::secure_storage;
use serde_json::json;

use super::{SECURE_STORAGE_KEY, SavedSshPasswords};
use crate::terminal::ssh::password_autofill::SshCredentialKey;

#[derive(Clone, Default)]
struct MemorySecureStorage {
    values: Rc<RefCell<HashMap<String, String>>>,
    fail_reads: bool,
}

impl MemorySecureStorage {
    fn stored(&self) -> Option<String> {
        self.values.borrow().get(SECURE_STORAGE_KEY).cloned()
    }

    fn store(&self, value: &str) {
        self.values
            .borrow_mut()
            .insert(SECURE_STORAGE_KEY.to_owned(), value.to_owned());
    }
}

impl secure_storage::SecureStorage for MemorySecureStorage {
    fn write_value(&self, key: &str, value: &str) -> Result<(), secure_storage::Error> {
        self.values
            .borrow_mut()
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn read_value(&self, key: &str) -> Result<String, secure_storage::Error> {
        if self.fail_reads {
            return Err(secure_storage::Error::Unknown(anyhow::anyhow!(
                "the keyring is locked"
            )));
        }
        self.values
            .borrow()
            .get(key)
            .cloned()
            .ok_or(secure_storage::Error::NotFound)
    }

    fn remove_value(&self, key: &str) -> Result<(), secure_storage::Error> {
        self.values
            .borrow_mut()
            .remove(key)
            .map(|_| ())
            .ok_or(secure_storage::Error::NotFound)
    }
}

fn add_saved_passwords(
    app: &mut App,
    storage: &MemorySecureStorage,
) -> ModelHandle<SavedSshPasswords> {
    let storage = storage.clone();
    app.add_singleton_model(move |_| -> secure_storage::Model { Box::new(storage) });
    app.add_model(|_| SavedSshPasswords::default())
}

fn key(user: &str, host: &str) -> SshCredentialKey {
    SshCredentialKey::new(Some(user.to_owned()), host, None)
}

#[test]
fn saved_passwords_are_stored_and_reloaded() {
    App::test((), |mut app| async move {
        let storage = MemorySecureStorage::default();
        let saved_passwords = add_saved_passwords(&mut app, &storage);
        saved_passwords.update(&mut app, |saved_passwords, ctx| {
            assert!(saved_passwords.save(key("root", "10.0.0.1"), "secret".to_owned(), ctx));
            assert!(saved_passwords.save(key("admin", "10.0.0.2"), "other".to_owned(), ctx));
            assert!(!saved_passwords.save(key("root", "10.0.0.1"), "secret".to_owned(), ctx));
        });

        let stored: serde_json::Value =
            serde_json::from_str(&storage.stored().expect("passwords are stored")).unwrap();
        assert_eq!(
            stored,
            json!([
                { "user": "admin", "host": "10.0.0.2", "password": "other" },
                { "user": "root", "host": "10.0.0.1", "password": "secret" },
            ])
        );

        let reloaded = app.add_model(|_| SavedSshPasswords::default());
        reloaded.read(&app, |saved_passwords, ctx| {
            assert_eq!(
                saved_passwords.password(&key("root", "10.0.0.1"), ctx),
                Some("secret")
            );
            assert_eq!(
                saved_passwords.password(&key("root", "10.0.0.2"), ctx),
                None
            );
            assert_eq!(
                saved_passwords.accounts(ctx).collect::<Vec<_>>(),
                [&key("admin", "10.0.0.2"), &key("root", "10.0.0.1")]
            );
        });
    });
}

#[test]
fn saving_replaces_the_accounts_password() {
    App::test((), |mut app| async move {
        let storage = MemorySecureStorage::default();
        let saved_passwords = add_saved_passwords(&mut app, &storage);
        saved_passwords.update(&mut app, |saved_passwords, ctx| {
            assert!(saved_passwords.save(key("root", "10.0.0.1"), "old".to_owned(), ctx));
            assert!(saved_passwords.save(key("root", "10.0.0.1"), "new".to_owned(), ctx));
            assert_eq!(
                saved_passwords.password(&key("root", "10.0.0.1"), ctx),
                Some("new")
            );
        });
    });
}

#[test]
fn forgetting_the_last_password_removes_it_from_storage() {
    App::test((), |mut app| async move {
        let storage = MemorySecureStorage::default();
        let saved_passwords = add_saved_passwords(&mut app, &storage);
        saved_passwords.update(&mut app, |saved_passwords, ctx| {
            assert!(saved_passwords.save(key("root", "10.0.0.1"), "secret".to_owned(), ctx));
            assert!(saved_passwords.forget(&key("root", "10.0.0.1"), ctx));
            assert!(!saved_passwords.forget(&key("root", "10.0.0.1"), ctx));
            assert_eq!(saved_passwords.accounts(ctx).count(), 0);
        });
        assert_eq!(storage.stored(), None);
    });
}

#[test]
fn unreadable_storage_isnt_overwritten() {
    App::test((), |mut app| async move {
        let storage = MemorySecureStorage {
            fail_reads: true,
            ..Default::default()
        };
        storage.store("previously saved");
        let saved_passwords = add_saved_passwords(&mut app, &storage);
        saved_passwords.update(&mut app, |saved_passwords, ctx| {
            assert_eq!(
                saved_passwords.password(&key("root", "10.0.0.1"), ctx),
                None
            );
            assert!(!saved_passwords.save(key("root", "10.0.0.1"), "secret".to_owned(), ctx));
        });
        assert_eq!(storage.stored().as_deref(), Some("previously saved"));
    });
}

#[test]
fn corrupt_storage_is_replaced() {
    App::test((), |mut app| async move {
        let storage = MemorySecureStorage::default();
        storage.store("not json");
        let saved_passwords = add_saved_passwords(&mut app, &storage);
        saved_passwords.update(&mut app, |saved_passwords, ctx| {
            assert_eq!(saved_passwords.accounts(ctx).count(), 0);
            assert!(saved_passwords.save(key("root", "10.0.0.1"), "secret".to_owned(), ctx));
        });
        let reloaded = app.add_model(|_| SavedSshPasswords::default());
        reloaded.read(&app, |saved_passwords, ctx| {
            assert_eq!(
                saved_passwords.password(&key("root", "10.0.0.1"), ctx),
                Some("secret")
            );
        });
    });
}
