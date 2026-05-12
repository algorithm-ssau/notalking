use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Argon2, Params};
use rand_core::OsRng;

use crate::auth::{AuthError, PasswordService};

#[derive(Clone)]
pub struct ArgonPasswordService;

impl PasswordService for ArgonPasswordService {
    fn hash_password(&self, plain: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            Params::default(),
        );
        argon
            .hash_password(plain.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| AuthError::Internal)
    }

    fn verify_password(&self, plain: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed = PasswordHash::new(hash).map_err(|_| AuthError::Internal)?;
        Ok(Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok())
    }
}
