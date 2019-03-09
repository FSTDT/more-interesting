use ring::{digest, pbkdf2, rand};
use ring::rand::SecureRandom;

static DIGEST_ALG: &'static digest::Algorithm = &digest::SHA256;
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
const PBKDF2_ITER: u32 = 100_000;

fn get_salt(out: &mut [u8]) {
    assert_eq!(out.len(), CREDENTIAL_LEN);
    let r = rand::SystemRandom::new();
    r.fill(out).unwrap();
}

/// Passwords are stored with three byte fields:
///
///     [8 bit version (always 1 for now)] <> [256 bit password hash ] <> [256 bit salt]
///
/// We don't do the breaking up of them in the DB itself,
/// because we don't want to mess with migrating old hashes if this format needs to change.
struct Parts<'a> {
    version: &'a mut [u8],
    hash: &'a mut [u8],
    salt: &'a mut [u8],
}

fn parts(whole: &mut [u8]) -> Parts {
    let (version, whole_) = whole.split_at_mut(1);
    let (hash, salt) = whole_.split_at_mut(CREDENTIAL_LEN);
    assert_eq!(version.len(), 1);
    assert_eq!(hash.len(), CREDENTIAL_LEN);
    assert_eq!(salt.len(), CREDENTIAL_LEN);
    Parts {
        version, hash, salt
    }
}

/// A password hashing function.
/// The resulting "hash" includes a salt and a version number,
/// so it can be upgraded without having to screw with any of the old passwords.
pub fn password_hash(password: &str) -> Vec<u8> {
    let mut result = vec![0; 1 + (2 * CREDENTIAL_LEN)];
    let parts = parts(&mut result[..]);
    parts.version[0] = 1;
    get_salt(parts.salt);
    pbkdf2::derive(
        DIGEST_ALG,
        PBKDF2_ITER,
        parts.salt,
        password.as_bytes(),
        parts.hash,
    );
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PasswordResult {
    Passed,
    Failed,
}

/// A password verification function.
/// The provided "hash" includes a salt and a version number,
/// so it can be upgraded without having to screw with any of the old passwords.
/// Do not attempt to compare the hash for equality yourself;
/// that's a great way to get timing attacks.
pub fn password_verify(password: &str, password_hash: &mut [u8]) -> PasswordResult {
    match password_hash[0] {
        1 => {
            let parts = parts(password_hash);
            let result = pbkdf2::verify(
                DIGEST_ALG,
                PBKDF2_ITER,
                parts.salt,
                password.as_bytes(),
                parts.hash,
            );
            if result.is_err() {
                PasswordResult::Failed
            } else {
                PasswordResult::Passed
            }
        }
        _ => panic!("unknown password hash version?!"),
    }
}
