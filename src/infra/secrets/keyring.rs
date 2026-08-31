//! The instance key on disk, and the versioned data keys it wraps.
//!
//! The Credential contract asks for four things this module owns, and they fit
//! together in one sentence: *the instance key wraps versioned data keys, every
//! ciphertext names the key that made it, new writes use the current key, and
//! older keys stay for reads.*
//!
//! ```text
//! <data-root>/platform/credential-key        catalog.db: credential_keys
//! ┌──────────────────────────────┐           ┌───────────────────────────┐
//! │ zfk1:<instance key>   (0600) │ ────────► │ 1  zfw1:<data key 1>      │
//! │ zfk1:<superseded, rekey only>│  unwraps  │ 2  zfw1:<data key 2>  ◄── │ current
//! └──────────────────────────────┘           └───────────────────────────┘
//!                                                        │ seals
//!                                                        ▼
//!                                     project_credentials.secret_json
//!                                     "zfc1:2:<nonce ‖ ciphertext ‖ tag>"
//! ```
//!
//! ## Why the data keys live in the database
//!
//! The contract forbids "the key stored in the database beside the ciphertext",
//! and this is not that: a `zfw1:` row is useless to anyone who does not also
//! hold the file the tree above draws on the left. It is Grafana's envelope
//! shape, and it buys the one thing a single file cannot give.
//!
//! **It is what tells first boot apart from a lost key.** A key file that also
//! held the keyring would be indistinguishable, when absent, from an instance
//! that has never started — so a deleted key, or a database restored without
//! the file beside it, would be answered by generating a new one and treating
//! every stored credential as unreadable. That is Jenkins' failure exactly:
//! "regenerates the key, treats ciphertext as plaintext, and fails much later
//! at an API call with no cryptographic error."
//!
//! With the rows in the catalog the question has an answer that is a fact and
//! not a guess:
//!
//! | Rows | Key | Outcome |
//! | --- | --- | --- |
//! | none | none | first boot — generate both |
//! | none | present | adopt the key, mint data key 1 |
//! | present | none | **refuse to start**: this instance had a key and does not now |
//! | present | wrong | **refuse to start**: the AEAD says so, loudly |
//!
//! Note the third row does not ask whether any credential exists. The keyring,
//! not the credential table, is the evidence a key was ever here — so an
//! instance restored without its key file refuses even while it holds nothing
//! to decrypt, which is the moment an operator can still go and find the file.
//!
//! ## Why the key file may hold more than one key
//!
//! `rekey` re-wraps every data key under a new instance key. Two writes have to
//! land — the file and the rows — and a crash between them must not be able to
//! produce rows nothing on disk can open. So the file is a list, newest first,
//! and unwrapping tries each in turn:
//!
//! 1. write the file as `[new, old]` — both keys now open something;
//! 2. re-wrap every row under `new`, in one transaction;
//! 3. write the file as `[new]`.
//!
//! A crash at any point leaves a state the next start can read. Only step 3
//! removes anything, and by then nothing needs it.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use super::envelope::{
    SecretCryptoError, SecretKey, open_secret, parse_secret_envelope, seal_secret, unwrap_data_key,
    wrap_data_key,
};

/// Where an instance keeps the key that wraps its data keys.
///
/// `platform/` and mode 0600, beside `cluster-signing-key` and for the same
/// reasons: STORE tier, instance-local, read on every start. It is deliberately
/// a *separate* file from the office join token — that token is rewritten
/// whenever an office is re-issued one, while this key must outlive every
/// membership change, because losing it costs every credential the instance
/// holds. One file, one lifecycle.
pub const CREDENTIAL_KEY_REL: &str = "platform/credential-key";

/// The variable that supplies an instance key instead of the file.
pub const CREDENTIAL_KEY_VAR: &str = "ZEBFLOW_CREDENTIAL_KEY";

/// One row of the `credential_keys` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedDataKey {
    /// Generation number; the largest is the one new writes use.
    pub key_id: u32,
    /// `zfw1:<base64url>` — the data key sealed under an instance key.
    pub wrapped: String,
    /// When this generation was minted.
    pub created_at: i64,
}

/// Where the instance key came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    /// `<data-root>/platform/credential-key`.
    File,
    /// [`CREDENTIAL_KEY_VAR`].
    Environment,
}

impl KeySource {
    /// How this source is named in an operator-facing message.
    pub fn describe(&self, path: &Path) -> String {
        match self {
            Self::File => format!("'{}'", path.display()),
            Self::Environment => CREDENTIAL_KEY_VAR.to_string(),
        }
    }
}

/// Why a keyring could not be opened or used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialKeyError {
    /// This instance has wrapped data keys and no instance key to open them.
    Missing {
        /// Where the key should be.
        path: PathBuf,
        /// How many generations are waiting for it.
        generations: usize,
    },
    /// A key is present and does not open what is stored.
    Unusable {
        /// What went wrong.
        reason: String,
        /// Where the key that failed came from.
        source: String,
    },
    /// The variable and the file both name a key, and they disagree.
    Mismatch {
        /// Where the stored key is.
        path: PathBuf,
    },
    /// The key file could not be read or written.
    Io {
        /// Which path.
        path: PathBuf,
        /// What the operating system said.
        reason: String,
    },
    /// A ciphertext names a generation this keyring does not hold.
    UnknownGeneration {
        /// The `key_id` inside the envelope.
        key_id: u32,
    },
    /// The system random source is unavailable.
    NoRandomness,
    /// `rekey` was asked for while the key comes from the environment.
    RekeyNotOwned,
}

/// The sentence every "the key is gone" message ends with.
const RECOVERY: &str = "Restore the key file from your backup, or set \
                        ZEBFLOW_CREDENTIAL_KEY to the key it held. A credential cannot be \
                        regenerated, so this instance will not start a new key over the old \
                        one — the credentials it holds would become permanently unreadable.";

impl fmt::Display for CredentialKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { path, generations } => write!(
                f,
                "this instance has {generations} credential encryption key generation(s) recorded \
                 in its catalog and no instance key at '{}'. {RECOVERY}",
                path.display()
            ),
            Self::Unusable { reason, source } => write!(
                f,
                "the credential encryption key from {source} is unusable ({reason}). {RECOVERY}"
            ),
            Self::Mismatch { path } => write!(
                f,
                "{CREDENTIAL_KEY_VAR} and '{}' name different credential encryption keys. Neither \
                 is overwritten by the other: one of them opens this instance's credentials and \
                 the other does not. Unset the variable to use the stored key, or move the stored \
                 file aside once you are certain the variable holds the right one.",
                path.display()
            ),
            Self::Io { path, reason } => {
                write!(
                    f,
                    "credential encryption key '{}': {reason}",
                    path.display()
                )
            }
            Self::UnknownGeneration { key_id } => write!(
                f,
                "a stored credential names encryption key generation {key_id}, which this \
                 instance's keyring does not hold. {RECOVERY}"
            ),
            Self::NoRandomness => {
                write!(f, "the system random source is unavailable")
            }
            Self::RekeyNotOwned => write!(
                f,
                "the instance key comes from {CREDENTIAL_KEY_VAR}, so this instance does not own \
                 the file a rekey would rewrite. Unset the variable and restart with the key \
                 stored at '{CREDENTIAL_KEY_REL}' first."
            ),
        }
    }
}

impl std::error::Error for CredentialKeyError {}

impl CredentialKeyError {
    fn unusable(source: KeySource, path: &Path, err: SecretCryptoError) -> Self {
        Self::Unusable {
            reason: err.to_string(),
            source: source.describe(path),
        }
    }
}

/// What [`CredentialKeyring::open`] produced, and what the caller must persist
/// before using it.
#[derive(Debug)]
pub struct KeyringBootstrap {
    /// The keyring itself.
    pub keyring: CredentialKeyring,
    /// A first generation that has to be written to the catalog.
    ///
    /// `Some` exactly when the instance had no keyring at all. The caller
    /// writes it and the keyring is then in agreement with storage; there is no
    /// path where a key is used before the row that names it exists.
    pub insert: Option<WrappedDataKey>,
}

/// An instance key, the data keys it wraps, and which of them is current.
#[derive(Debug)]
pub struct CredentialKeyring {
    key_file: PathBuf,
    source: KeySource,
    /// Newest first. Only the first wraps new data keys; the rest exist so an
    /// interrupted rekey is still readable.
    instance_keys: RwLock<Vec<SecretKey>>,
    keys: RwLock<BTreeMap<u32, SecretKey>>,
}

impl CredentialKeyring {
    /// Open this instance's keyring, generating one only on a genuine first boot.
    ///
    /// `env_value` is [`CREDENTIAL_KEY_VAR`], already trimmed of the
    /// blank-is-unset case; `stored` is every `credential_keys` row.
    pub fn open(
        key_file: &Path,
        env_value: Option<&str>,
        stored: &[WrappedDataKey],
        now: i64,
    ) -> Result<KeyringBootstrap, CredentialKeyError> {
        let file_keys = read_key_file(key_file)?;
        let env_key = match env_value {
            Some(value) => Some(SecretKey::parse(value).map_err(|err| {
                CredentialKeyError::unusable(KeySource::Environment, key_file, err)
            })?),
            None => None,
        };

        // A variable and a file that disagree is a refusal, never a silent
        // overwrite in either direction — the rule `offices.md` §8 states for
        // the join token and names this contract as its source.
        if let (Some(env), Some(first)) = (env_key.as_ref(), file_keys.first())
            && env != first
        {
            return Err(CredentialKeyError::Mismatch {
                path: key_file.to_path_buf(),
            });
        }

        let (instance_keys, source) = match (env_key, file_keys.is_empty()) {
            (Some(env), _) => (vec![env], KeySource::Environment),
            (None, false) => (file_keys, KeySource::File),
            (None, true) => {
                if !stored.is_empty() {
                    return Err(CredentialKeyError::Missing {
                        path: key_file.to_path_buf(),
                        generations: stored.len(),
                    });
                }
                let generated =
                    SecretKey::generate().map_err(|_| CredentialKeyError::NoRandomness)?;
                write_key_file(key_file, std::slice::from_ref(&generated))?;
                (vec![generated], KeySource::File)
            }
        };

        let keyring = Self {
            key_file: key_file.to_path_buf(),
            source,
            instance_keys: RwLock::new(instance_keys),
            keys: RwLock::new(BTreeMap::new()),
        };

        if stored.is_empty() {
            let (row, key) = keyring.mint(1, now)?;
            keyring.install(1, key);
            return Ok(KeyringBootstrap {
                keyring,
                insert: Some(row),
            });
        }

        for row in stored {
            let key = keyring.unwrap_any(row)?;
            keyring.install(row.key_id, key);
        }
        Ok(KeyringBootstrap {
            keyring,
            insert: None,
        })
    }

    /// Try every instance key this file holds, newest first.
    ///
    /// More than one only during a rekey; the loop is what makes an
    /// interrupted one readable rather than fatal.
    fn unwrap_any(&self, row: &WrappedDataKey) -> Result<SecretKey, CredentialKeyError> {
        let instance_keys = self.instance_keys.read().unwrap_or_else(|e| e.into_inner());
        let mut last = None;
        for instance in instance_keys.iter() {
            match unwrap_data_key(instance, row.key_id, &row.wrapped) {
                Ok(key) => return Ok(key),
                Err(err) => last = Some(err),
            }
        }
        Err(CredentialKeyError::unusable(
            self.source,
            &self.key_file,
            last.unwrap_or(SecretCryptoError::WrongKey { envelope: "zfw1" }),
        ))
    }

    fn mint(
        &self,
        key_id: u32,
        now: i64,
    ) -> Result<(WrappedDataKey, SecretKey), CredentialKeyError> {
        let data = SecretKey::generate().map_err(|_| CredentialKeyError::NoRandomness)?;
        let instance_keys = self.instance_keys.read().unwrap_or_else(|e| e.into_inner());
        let primary = instance_keys
            .first()
            .expect("a keyring always holds at least one instance key");
        let wrapped = wrap_data_key(primary, key_id, &data)
            .map_err(|err| CredentialKeyError::unusable(self.source, &self.key_file, err))?;
        Ok((
            WrappedDataKey {
                key_id,
                wrapped,
                created_at: now,
            },
            data,
        ))
    }

    fn install(&self, key_id: u32, key: SecretKey) {
        self.keys
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key_id, key);
    }

    /// The generation new writes use: the largest this keyring holds.
    pub fn current_key_id(&self) -> u32 {
        self.keys
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0)
    }

    /// Every generation this keyring holds, oldest first.
    pub fn key_ids(&self) -> Vec<u32> {
        self.keys
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .copied()
            .collect()
    }

    /// Where the instance key came from.
    pub fn source(&self) -> KeySource {
        self.source
    }

    /// Seal one secret half under the current generation.
    pub fn seal(&self, plaintext: &str) -> Result<String, CredentialKeyError> {
        let key_id = self.current_key_id();
        let keys = self.keys.read().unwrap_or_else(|e| e.into_inner());
        let key = keys
            .get(&key_id)
            .ok_or(CredentialKeyError::UnknownGeneration { key_id })?;
        seal_secret(key, key_id, plaintext)
            .map_err(|err| CredentialKeyError::unusable(self.source, &self.key_file, err))
    }

    /// Open one `zfc1:` envelope, under whichever generation it names.
    pub fn open_envelope(&self, envelope: &str) -> Result<String, CredentialKeyError> {
        let Some((key_id, body)) = parse_secret_envelope(envelope) else {
            return Err(CredentialKeyError::Unusable {
                reason: "the stored value is not a 'zfc1' envelope".to_string(),
                source: self.source.describe(&self.key_file),
            });
        };
        let keys = self.keys.read().unwrap_or_else(|e| e.into_inner());
        let key = keys
            .get(&key_id)
            .ok_or(CredentialKeyError::UnknownGeneration { key_id })?;
        open_secret(key, key_id, body)
            .map_err(|err| CredentialKeyError::unusable(self.source, &self.key_file, err))
    }

    /// Mint the next generation, for the caller to persist.
    ///
    /// The row is returned rather than installed: a data key that is in memory
    /// and not in the catalog would seal credentials nothing could ever open
    /// again. The caller writes the row, then calls [`Self::adopt`].
    pub fn prepare_rotation(
        &self,
        now: i64,
    ) -> Result<(WrappedDataKey, SecretKey), CredentialKeyError> {
        self.mint(self.current_key_id() + 1, now)
    }

    /// Install a generation whose row is already in the catalog.
    pub fn adopt(&self, key_id: u32, key: SecretKey) {
        self.install(key_id, key);
    }

    /// Step 1 of a rekey: a new instance key, every data key re-wrapped under
    /// it, and the file already holding both keys.
    ///
    /// On return the file is `[new, old…]`, so the rows the caller is about to
    /// write are openable whether or not the caller survives writing them.
    pub fn begin_rekey(&self) -> Result<Vec<WrappedDataKey>, CredentialKeyError> {
        if self.source == KeySource::Environment {
            return Err(CredentialKeyError::RekeyNotOwned);
        }
        let fresh = SecretKey::generate().map_err(|_| CredentialKeyError::NoRandomness)?;
        let rewrapped = {
            let keys = self.keys.read().unwrap_or_else(|e| e.into_inner());
            keys.iter()
                .map(|(key_id, key)| {
                    wrap_data_key(&fresh, *key_id, key)
                        .map(|wrapped| WrappedDataKey {
                            key_id: *key_id,
                            wrapped,
                            created_at: 0,
                        })
                        .map_err(|err| {
                            CredentialKeyError::unusable(self.source, &self.key_file, err)
                        })
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let mut instance_keys = self
            .instance_keys
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let mut both = vec![fresh];
        both.extend(instance_keys.iter().cloned());
        write_key_file(&self.key_file, &both)?;
        *instance_keys = both;
        Ok(rewrapped)
    }

    /// Step 3 of a rekey: the rows are written, so the superseded keys go.
    pub fn finish_rekey(&self) -> Result<(), CredentialKeyError> {
        let mut instance_keys = self
            .instance_keys
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let primary = vec![
            instance_keys
                .first()
                .expect("a keyring always holds at least one instance key")
                .clone(),
        ];
        write_key_file(&self.key_file, &primary)?;
        *instance_keys = primary;
        Ok(())
    }
}

/// Read the key file, newest key first. An absent file is an empty list; an
/// unreadable or malformed one is an error, and never an absent one.
fn read_key_file(path: &Path) -> Result<Vec<SecretKey>, CredentialKeyError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(CredentialKeyError::Io {
                path: path.to_path_buf(),
                reason: format!("failed reading: {err}"),
            });
        }
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            SecretKey::parse(line)
                .map_err(|err| CredentialKeyError::unusable(KeySource::File, path, err))
        })
        .collect()
}

/// Write the key file, mode 0600, newest key first.
///
/// The same shape `write_private_file` in the cluster join-token service uses:
/// `mode` at open time and `set_permissions` after, because an existing file
/// keeps its old mode through `create`.
fn write_key_file(path: &Path, keys: &[SecretKey]) -> Result<(), CredentialKeyError> {
    let io = |reason: String| CredentialKeyError::Io {
        path: path.to_path_buf(),
        reason,
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| io(format!("failed creating parent: {err}")))?;
    }
    let mut body = String::new();
    for key in keys {
        body.push_str(&key.render());
        body.push('\n');
    }
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|err| io(format!("failed creating: {err}")))?;
    file.write_all(body.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|err| io(format!("failed writing: {err}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|err| io(format!("failed securing: {err}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "zebflow-credential-keyring-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("platform")).expect("temp root");
        root
    }

    fn key_path(root: &Path) -> PathBuf {
        root.join(CREDENTIAL_KEY_REL)
    }

    #[test]
    fn first_boot_writes_a_private_key_file_and_one_generation() {
        let root = temp_root("first-boot");
        let path = key_path(&root);
        let boot = CredentialKeyring::open(&path, None, &[], 100).expect("first boot");
        let row = boot.insert.expect("a first generation is minted");
        assert_eq!(row.key_id, 1);
        assert!(row.wrapped.starts_with("zfw1:"));
        assert_eq!(row.created_at, 100);
        assert_eq!(boot.keyring.current_key_id(), 1);
        assert_eq!(boot.keyring.source(), KeySource::File);

        assert!(path.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "an instance key is not a world-readable file");
        }

        // A second start reads what the first wrote and mints nothing.
        let again = CredentialKeyring::open(&path, None, &[row], 200).expect("second boot");
        assert!(again.insert.is_none());
        assert_eq!(again.keyring.current_key_id(), 1);

        // The two starts agree about the ciphertext.
        let sealed = boot.keyring.seal("hunter2").expect("seal");
        assert_eq!(
            again.keyring.open_envelope(&sealed).expect("open"),
            "hunter2"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_removed_key_refuses_rather_than_regenerating() {
        let root = temp_root("removed-key");
        let path = key_path(&root);
        let boot = CredentialKeyring::open(&path, None, &[], 1).expect("first boot");
        let row = boot.insert.expect("row");
        fs::remove_file(&path).expect("remove the key");

        let err = CredentialKeyring::open(&path, None, &[row.clone()], 2).expect_err("must refuse");
        assert!(
            matches!(err, CredentialKeyError::Missing { generations: 1, .. }),
            "{err:?}"
        );
        let message = err.to_string();
        assert!(message.contains("cannot be regenerated"), "{message}");
        assert!(message.contains(CREDENTIAL_KEY_VAR), "{message}");
        assert!(
            !path.exists(),
            "a refusal must not have written a replacement key"
        );

        // An instance holding no credentials at all refuses just the same: the
        // keyring is the evidence, not the credential table.
        assert!(CredentialKeyring::open(&path, None, &[row], 3).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_wrong_key_refuses_loudly_and_a_damaged_one_is_not_an_absent_one() {
        let root = temp_root("wrong-key");
        let path = key_path(&root);
        let boot = CredentialKeyring::open(&path, None, &[], 1).expect("first boot");
        let row = boot.insert.expect("row");

        // Somebody else's key, correctly formed.
        let other = SecretKey::generate().expect("generate");
        write_key_file(&path, std::slice::from_ref(&other)).expect("write");
        let err = CredentialKeyring::open(&path, None, &[row.clone()], 2).expect_err("wrong key");
        assert!(
            matches!(err, CredentialKeyError::Unusable { .. }),
            "{err:?}"
        );
        assert!(err.to_string().contains("did not authenticate"), "{err}");

        // Garbage in the file is a refusal too, never a fresh key.
        fs::write(&path, b"not a key\n").expect("damage");
        let err = CredentialKeyring::open(&path, None, &[row], 3).expect_err("damaged");
        assert!(
            matches!(err, CredentialKeyError::Unusable { .. }),
            "{err:?}"
        );
        assert_eq!(
            fs::read_to_string(&path).expect("read"),
            "not a key\n",
            "a refusal must never overwrite the file it refused"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_variable_and_the_file_disagreeing_refuses_and_overwrites_neither() {
        let root = temp_root("mismatch");
        let path = key_path(&root);
        let boot = CredentialKeyring::open(&path, None, &[], 1).expect("first boot");
        let row = boot.insert.expect("row");
        let stored = fs::read_to_string(&path).expect("read");

        let other = SecretKey::generate().expect("generate").render();
        let err = CredentialKeyring::open(&path, Some(&other), &[row.clone()], 2)
            .expect_err("disagreement");
        assert!(
            matches!(err, CredentialKeyError::Mismatch { .. }),
            "{err:?}"
        );
        assert_eq!(fs::read_to_string(&path).expect("read"), stored);

        // The same key stated twice is agreement, not a conflict.
        let same = stored.trim().to_string();
        let opened = CredentialKeyring::open(&path, Some(&same), &[row], 3).expect("agreement");
        assert_eq!(opened.keyring.source(), KeySource::Environment);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_variable_supplies_a_key_on_an_instance_with_no_file() {
        let root = temp_root("env-only");
        let path = key_path(&root);
        let supplied = SecretKey::generate().expect("generate").render();
        let boot = CredentialKeyring::open(&path, Some(&supplied), &[], 1).expect("env boot");
        assert_eq!(boot.keyring.source(), KeySource::Environment);
        assert!(
            !path.exists(),
            "a key supplied by variable is not written to disk behind the operator's back"
        );
        // And it cannot be rekeyed, because this instance does not own it.
        assert_eq!(
            boot.keyring.begin_rekey(),
            Err(CredentialKeyError::RekeyNotOwned)
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rotation_adds_a_generation_and_leaves_the_old_one_readable() {
        let root = temp_root("rotate");
        let path = key_path(&root);
        let boot = CredentialKeyring::open(&path, None, &[], 1).expect("first boot");
        let keyring = boot.keyring;
        let first_row = boot.insert.expect("row");

        let old = keyring.seal("written under generation one").expect("seal");
        assert!(old.starts_with("zfc1:1:"));

        let (row, key) = keyring.prepare_rotation(50).expect("rotate");
        assert_eq!(row.key_id, 2);
        keyring.adopt(row.key_id, key);
        assert_eq!(keyring.current_key_id(), 2);

        let new = keyring.seal("written under generation two").expect("seal");
        assert!(new.starts_with("zfc1:2:"), "new writes use the new key");
        assert_eq!(
            keyring.open_envelope(&old).expect("old still readable"),
            "written under generation one"
        );

        // And a restart holding both rows reaches the same place.
        drop(keyring);
        let restarted = CredentialKeyring::open(&path, None, &[first_row, row], 60)
            .expect("restart")
            .keyring;
        assert_eq!(restarted.key_ids(), vec![1, 2]);
        assert_eq!(restarted.current_key_id(), 2);
        assert_eq!(
            restarted.open_envelope(&old).expect("old"),
            "written under generation one"
        );
        assert_eq!(
            restarted.open_envelope(&new).expect("new"),
            "written under generation two"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_rekey_interrupted_between_its_two_writes_is_still_readable() {
        let root = temp_root("rekey-crash");
        let path = key_path(&root);
        let boot = CredentialKeyring::open(&path, None, &[], 1).expect("first boot");
        let keyring = boot.keyring;
        let old_row = boot.insert.expect("row");
        let sealed = keyring.seal("survives a rekey").expect("seal");

        let rewrapped = keyring.begin_rekey().expect("begin");
        assert_eq!(rewrapped.len(), 1);
        assert_ne!(rewrapped[0].wrapped, old_row.wrapped);
        assert_eq!(
            read_key_file(&path).expect("read").len(),
            2,
            "both keys are on disk while the rows are being written"
        );

        // Crash here: the rows were never written, so the catalog still holds
        // the old wrapping. The next start opens it with the second key.
        drop(keyring);
        let recovered = CredentialKeyring::open(&path, None, &[old_row], 2)
            .expect("an interrupted rekey still starts")
            .keyring;
        assert_eq!(
            recovered.open_envelope(&sealed).expect("open"),
            "survives a rekey"
        );

        // Crash the other way: the rows landed and the file was never trimmed.
        let recovered = CredentialKeyring::open(&path, None, &rewrapped, 3)
            .expect("the other interruption starts too")
            .keyring;
        assert_eq!(
            recovered.open_envelope(&sealed).expect("open"),
            "survives a rekey"
        );

        // Finishing trims the file to the key everything is now wrapped under.
        let rewrapped2 = recovered.begin_rekey().expect("begin");
        recovered.finish_rekey().expect("finish");
        assert_eq!(read_key_file(&path).expect("read").len(), 1);
        let after = CredentialKeyring::open(&path, None, &rewrapped2, 4)
            .expect("after")
            .keyring;
        assert_eq!(
            after.open_envelope(&sealed).expect("open"),
            "survives a rekey"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_ciphertext_naming_a_generation_the_keyring_lost_is_refused_by_name() {
        let root = temp_root("unknown-generation");
        let path = key_path(&root);
        let keyring = CredentialKeyring::open(&path, None, &[], 1)
            .expect("boot")
            .keyring;
        let err = keyring
            .open_envelope("zfc1:9:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")
            .expect_err("unknown generation");
        assert_eq!(err, CredentialKeyError::UnknownGeneration { key_id: 9 });
        assert!(err.to_string().contains("generation 9"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }
}
