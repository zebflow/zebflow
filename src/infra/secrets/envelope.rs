//! The self-describing ciphertext the Credential contract requires.
//!
//! Three envelopes, one AEAD, and a rule that every one of them names the
//! format that produced it:
//!
//! | Envelope | Holds | Where it lives |
//! | --- | --- | --- |
//! | `zfk1:<key>` | an instance key, in the clear | `<data-root>/platform/credential-key`, mode 0600 |
//! | `zfw1:<blob>` | a data key, wrapped under an instance key | `credential_keys.wrapped_key` in the catalog |
//! | `zfc1:<key_id>:<blob>` | one credential's secret half | `project_credentials.secret_json` |
//!
//! The contract's reason for the tag is worth restating, because it is the
//! whole point of writing four bytes nobody reads: *"a reader that meets an
//! older format tag knows what it is holding instead of guessing from its own
//! version"*. A bare blob is readable exactly once, by the code that wrote it.
//!
//! ## Why ChaCha20-Poly1305, and why `ring`
//!
//! The contract requires an AEAD — "the ciphertext authenticates, so a wrong
//! key fails loudly instead of yielding garbage" — and *recommends*
//! XChaCha20-Poly1305 for `zfc1`, while stating plainly that "the contract
//! requires the shape; the named cipher is a recommendation".
//!
//! `zfc1` here is **ChaCha20-Poly1305 (RFC 8439, 96-bit nonce)** from `ring`.
//! The recommendation is followed in the part that carries the reasoning and
//! departed from in the part that does not:
//!
//! - **The reason given for ChaCha is met exactly.** "Constant-time in pure
//!   software, so it does not depend on AES hardware a Raspberry Pi may not
//!   have" is a property of the ChaCha20 core and of Poly1305, not of the
//!   XChaCha nonce extension. A Pi 4's Cortex-A72 has no AES instructions, and
//!   this cipher does not want any. AES-256-GCM was rejected for that reason
//!   and no other.
//! - **The reason given for the large nonce is met by arithmetic instead.**
//!   XChaCha's 192-bit nonce exists so random nonces need no counter. At 96
//!   bits, a birthday collision among `n` random nonces under one data key has
//!   probability about `n² / 2⁹⁷`. A busy instance that rewrites a credential
//!   every second for a decade reaches `n ≈ 2²⁸`, for a collision chance near
//!   `2⁻⁴¹`. Rotation — which the contract requires from the first release —
//!   resets `n` to zero whenever an operator wants it lower still. There is no
//!   counter to keep here either.
//! - **`ring` is already in this build's dependency graph**, and
//!   `infra/cluster/security/controller_key.rs` chose it over `ed25519-dalek`
//!   for exactly that: "naming it in `Cargo.toml` adds a line and not a crate".
//!   The `chacha20poly1305` crate would have added `poly1305`, `aead`, and
//!   `universal-hash` — a tree that is not here today — to reach a nonce size
//!   the paragraph above does not need.
//!
//! If that trade ever stops being the right one, the tag is why it costs
//! nothing: `zfc2` can be XChaCha20-Poly1305 and every `zfc1` byte on disk
//! stays readable.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ring::aead::{Aad, CHACHA20_POLY1305, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};

/// Format word for an instance key at rest.
pub const INSTANCE_KEY_TAG: &str = "zfk1";
/// Format word for a data key wrapped under an instance key.
pub const WRAPPED_KEY_TAG: &str = "zfw1";
/// Format word for one encrypted secret half.
pub const SECRET_TAG: &str = "zfc1";

/// Key length for every envelope above.
pub const KEY_LEN: usize = 32;

/// Why a value could not be sealed, opened, or parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretCryptoError {
    /// The value is not the envelope it claims to be.
    Malformed {
        /// Which envelope was expected.
        envelope: &'static str,
        /// What was wrong with the value.
        reason: String,
    },
    /// The envelope is well formed and the key does not open it.
    ///
    /// This is the loud failure an AEAD exists to give: a wrong key never
    /// yields plausible garbage.
    WrongKey {
        /// Which envelope failed to authenticate.
        envelope: &'static str,
    },
    /// The operating system would not produce random bytes.
    NoRandomness,
}

impl fmt::Display for SecretCryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed { envelope, reason } => {
                write!(f, "not a usable '{envelope}' value ({reason})")
            }
            Self::WrongKey { envelope } => write!(
                f,
                "a '{envelope}' value did not authenticate under this key — the key is the wrong \
                 one, or the stored bytes were altered"
            ),
            Self::NoRandomness => write!(f, "the system random source is unavailable"),
        }
    }
}

impl std::error::Error for SecretCryptoError {}

/// Thirty-two secret bytes, an instance key or a data key.
///
/// Neither `Debug` nor `Display` renders the bytes, and `Drop` overwrites them:
/// the contract's threat model does not include process memory, but leaving a
/// key in a freed allocation for no reason is not a boundary anyone chose.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretKey([u8; KEY_LEN]);

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey(…)")
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        // Volatile so the write is not optimised away as dead.
        for byte in &mut self.0 {
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
    }
}

impl SecretKey {
    /// A fresh key from the system random source.
    pub fn generate() -> Result<Self, SecretCryptoError> {
        let mut bytes = [0u8; KEY_LEN];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| SecretCryptoError::NoRandomness)?;
        Ok(Self(bytes))
    }

    /// A key from bytes that are already secret.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    fn aead(&self) -> LessSafeKey {
        // `expect` cannot fire: the slice is exactly `CHACHA20_POLY1305.key_len()`.
        LessSafeKey::new(
            UnboundKey::new(&CHACHA20_POLY1305, &self.0).expect("32-byte chacha20-poly1305 key"),
        )
    }

    /// `base64url(nonce ‖ ciphertext ‖ tag)` for one plaintext under one AAD.
    fn seal(&self, aad: &str, plaintext: &[u8]) -> Result<String, SecretCryptoError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        SystemRandom::new()
            .fill(&mut nonce_bytes)
            .map_err(|_| SecretCryptoError::NoRandomness)?;
        let mut buffer = plaintext.to_vec();
        self.aead()
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce_bytes),
                Aad::from(aad.as_bytes()),
                &mut buffer,
            )
            .map_err(|_| SecretCryptoError::NoRandomness)?;
        let mut out = Vec::with_capacity(NONCE_LEN + buffer.len());
        out.extend_from_slice(&nonce_bytes);
        out.append(&mut buffer);
        Ok(URL_SAFE_NO_PAD.encode(out))
    }

    /// The inverse of [`Self::seal`]; a wrong key or altered bytes is an error.
    fn open(
        &self,
        envelope: &'static str,
        aad: &str,
        body: &str,
    ) -> Result<Vec<u8>, SecretCryptoError> {
        let raw =
            URL_SAFE_NO_PAD
                .decode(body.trim())
                .map_err(|err| SecretCryptoError::Malformed {
                    envelope,
                    reason: format!("body is not base64url: {err}"),
                })?;
        if raw.len() <= NONCE_LEN {
            return Err(SecretCryptoError::Malformed {
                envelope,
                reason: format!(
                    "body is {} bytes, shorter than a nonce and a tag",
                    raw.len()
                ),
            });
        }
        let (nonce_bytes, rest) = raw.split_at(NONCE_LEN);
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(nonce_bytes);
        let mut buffer = rest.to_vec();
        let opened = self
            .aead()
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(aad.as_bytes()),
                &mut buffer,
            )
            .map_err(|_| SecretCryptoError::WrongKey { envelope })?;
        Ok(opened.to_vec())
    }

    /// This key as it is written to `platform/credential-key`.
    pub fn render(&self) -> String {
        format!("{INSTANCE_KEY_TAG}:{}", URL_SAFE_NO_PAD.encode(self.0))
    }

    /// Read one `zfk1:` line back.
    ///
    /// Anything else is malformed and is never treated as an absent key: the
    /// contract says "on a missing or wrong key, refuse to start", and a
    /// caller that could not tell the two apart would regenerate over the
    /// second one.
    pub fn parse(text: &str) -> Result<Self, SecretCryptoError> {
        let text = text.trim();
        let Some(body) = text.strip_prefix(&format!("{INSTANCE_KEY_TAG}:")) else {
            let word = text.split(':').next().unwrap_or("");
            return Err(SecretCryptoError::Malformed {
                envelope: INSTANCE_KEY_TAG,
                reason: if word.is_empty() {
                    "the value is empty".to_string()
                } else {
                    format!("format word is '{word}', not '{INSTANCE_KEY_TAG}'")
                },
            });
        };
        let raw =
            URL_SAFE_NO_PAD
                .decode(body.trim())
                .map_err(|err| SecretCryptoError::Malformed {
                    envelope: INSTANCE_KEY_TAG,
                    reason: format!("key is not base64url: {err}"),
                })?;
        if raw.len() != KEY_LEN {
            return Err(SecretCryptoError::Malformed {
                envelope: INSTANCE_KEY_TAG,
                reason: format!("key is {} bytes, not {KEY_LEN}", raw.len()),
            });
        }
        let mut bytes = [0u8; KEY_LEN];
        bytes.copy_from_slice(&raw);
        Ok(Self(bytes))
    }
}

/// What a wrapped data key is bound to.
///
/// The slot is authenticated, so a wrapped key cannot be moved to another
/// `key_id` and quietly become a different generation.
fn wrap_aad(key_id: u32) -> String {
    format!("zebflow/credential-keyring/{WRAPPED_KEY_TAG}/{key_id}")
}

/// What one secret envelope is bound to.
///
/// The key id and the format word, and deliberately **not** the row's
/// `owner/project/credential_id`. Binding the row would stop a ciphertext
/// being copied between rows by somebody with write access to the catalog —
/// an attacker the contract explicitly places outside this feature's reach
/// ("it does not protect against filesystem access") — at the price of
/// `transfer_project_owner`, which rewrites `owner` on `project_credentials`
/// with a plain `UPDATE` and would silently strand every credential in the
/// project it moved.
fn secret_aad(key_id: u32) -> String {
    format!("zebflow/credential/{SECRET_TAG}/{key_id}")
}

/// Seal one data key under an instance key, for the `credential_keys` table.
pub fn wrap_data_key(
    instance: &SecretKey,
    key_id: u32,
    data: &SecretKey,
) -> Result<String, SecretCryptoError> {
    let body = instance.seal(&wrap_aad(key_id), &data.0)?;
    Ok(format!("{WRAPPED_KEY_TAG}:{body}"))
}

/// Open one stored `zfw1:` value under an instance key.
pub fn unwrap_data_key(
    instance: &SecretKey,
    key_id: u32,
    wrapped: &str,
) -> Result<SecretKey, SecretCryptoError> {
    let Some(body) = wrapped.trim().strip_prefix(&format!("{WRAPPED_KEY_TAG}:")) else {
        let word = wrapped.trim().split(':').next().unwrap_or("");
        return Err(SecretCryptoError::Malformed {
            envelope: WRAPPED_KEY_TAG,
            reason: format!("format word is '{word}', not '{WRAPPED_KEY_TAG}'"),
        });
    };
    let raw = instance.open(WRAPPED_KEY_TAG, &wrap_aad(key_id), body)?;
    if raw.len() != KEY_LEN {
        return Err(SecretCryptoError::Malformed {
            envelope: WRAPPED_KEY_TAG,
            reason: format!("wrapped key is {} bytes, not {KEY_LEN}", raw.len()),
        });
    }
    let mut bytes = [0u8; KEY_LEN];
    bytes.copy_from_slice(&raw);
    Ok(SecretKey::from_bytes(bytes))
}

/// Seal one secret half under a data key, naming the key that made it.
pub fn seal_secret(
    data_key: &SecretKey,
    key_id: u32,
    plaintext: &str,
) -> Result<String, SecretCryptoError> {
    let body = data_key.seal(&secret_aad(key_id), plaintext.as_bytes())?;
    Ok(format!("{SECRET_TAG}:{key_id}:{body}"))
}

/// Open one `zfc1:` envelope under the data key its `key_id` named.
pub fn open_secret(
    data_key: &SecretKey,
    key_id: u32,
    body: &str,
) -> Result<String, SecretCryptoError> {
    let raw = data_key.open(SECRET_TAG, &secret_aad(key_id), body)?;
    String::from_utf8(raw).map_err(|err| SecretCryptoError::Malformed {
        envelope: SECRET_TAG,
        reason: format!("plaintext is not utf-8: {err}"),
    })
}

/// Split `zfc1:<key_id>:<body>`, or `None` when this is not that envelope.
///
/// `None` is not an error: a row written before this feature existed holds
/// plaintext JSON, and the reader that meets one has to be able to say so.
pub fn parse_secret_envelope(text: &str) -> Option<(u32, &str)> {
    let rest = text.strip_prefix(&format!("{SECRET_TAG}:"))?;
    let (id, body) = rest.split_once(':')?;
    let key_id = id.parse::<u32>().ok()?;
    if body.is_empty() {
        return None;
    }
    Some((key_id, body))
}

/// Whether this text is a `zfc1:` envelope at all.
pub fn is_secret_envelope(text: &str) -> bool {
    parse_secret_envelope(text).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_round_trips_and_a_wrong_key_fails_loudly() {
        let key = SecretKey::generate().expect("generate");
        let sealed = seal_secret(&key, 1, r#"{"password":"hunter2"}"#).expect("seal");
        assert!(sealed.starts_with("zfc1:1:"));
        assert!(
            !sealed.contains("hunter2"),
            "the plaintext must not survive in the envelope: {sealed}"
        );

        let (key_id, body) = parse_secret_envelope(&sealed).expect("parse");
        assert_eq!(key_id, 1);
        assert_eq!(
            open_secret(&key, key_id, body).expect("open"),
            r#"{"password":"hunter2"}"#
        );

        // A different key is an authentication failure, never garbage.
        let other = SecretKey::generate().expect("generate");
        assert_eq!(
            open_secret(&other, key_id, body),
            Err(SecretCryptoError::WrongKey { envelope: "zfc1" })
        );
        // So is the right key under the wrong slot.
        assert_eq!(
            open_secret(&key, 2, body),
            Err(SecretCryptoError::WrongKey { envelope: "zfc1" })
        );
    }

    #[test]
    fn one_plaintext_never_seals_to_the_same_bytes_twice() {
        // A deterministic envelope would leak "these two credentials share a
        // password" from a stolen dump — Windmill's zero-IV failure.
        let key = SecretKey::generate().expect("generate");
        let first = seal_secret(&key, 1, "same").expect("seal");
        let second = seal_secret(&key, 1, "same").expect("seal");
        assert_ne!(first, second);
    }

    #[test]
    fn an_altered_envelope_is_refused_rather_than_decrypted() {
        let key = SecretKey::generate().expect("generate");
        let sealed = seal_secret(&key, 1, "value").expect("seal");
        let (key_id, body) = parse_secret_envelope(&sealed).expect("parse");
        let mut altered: Vec<char> = body.chars().collect();
        let last = altered.len() - 1;
        altered[last] = if altered[last] == 'A' { 'B' } else { 'A' };
        let altered: String = altered.into_iter().collect();
        assert!(open_secret(&key, key_id, &altered).is_err());
    }

    #[test]
    fn a_data_key_survives_wrapping_and_is_bound_to_its_slot() {
        let instance = SecretKey::generate().expect("generate");
        let data = SecretKey::generate().expect("generate");
        let wrapped = wrap_data_key(&instance, 3, &data).expect("wrap");
        assert!(wrapped.starts_with("zfw1:"));
        assert_eq!(
            unwrap_data_key(&instance, 3, &wrapped).expect("unwrap"),
            data
        );
        // The slot is authenticated: a row moved to another key_id is refused.
        assert_eq!(
            unwrap_data_key(&instance, 4, &wrapped),
            Err(SecretCryptoError::WrongKey { envelope: "zfw1" })
        );
        // So is another instance's key.
        let other = SecretKey::generate().expect("generate");
        assert_eq!(
            unwrap_data_key(&other, 3, &wrapped),
            Err(SecretCryptoError::WrongKey { envelope: "zfw1" })
        );
    }

    #[test]
    fn an_instance_key_round_trips_and_a_damaged_one_is_never_an_absent_one() {
        let key = SecretKey::generate().expect("generate");
        let rendered = key.render();
        assert!(rendered.starts_with("zfk1:"));
        assert_eq!(SecretKey::parse(&rendered).expect("parse"), key);
        assert_eq!(
            SecretKey::parse(&format!("  {rendered}\n")).expect("ws"),
            key
        );

        for damaged in ["", "hello", "zfk2:abc", "zfk1:!!!", "zfk1:AAAA"] {
            let err = SecretKey::parse(damaged).expect_err(damaged);
            assert!(
                matches!(err, SecretCryptoError::Malformed { .. }),
                "{damaged}: {err}"
            );
        }
    }

    #[test]
    fn a_legacy_plaintext_value_is_recognised_as_not_an_envelope() {
        assert!(!is_secret_envelope(r#"{"password":"hunter2"}"#));
        assert!(!is_secret_envelope("zfc1:"));
        assert!(!is_secret_envelope("zfc1:x:body"));
        assert!(!is_secret_envelope("zfc1:1:"));
        assert!(is_secret_envelope("zfc1:1:body"));
    }
}
