use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("unauthorized: only the owner may do this")]
    Unauthorized {},

    #[error("unauthorized: sender is not an allowlisted issuer")]
    NotIssuer {},

    #[error("certificate {cert_id} already exists")]
    DuplicateCertificate { cert_id: String },

    #[error("certificate {cert_id} not found")]
    CertificateNotFound { cert_id: String },

    #[error("certificate {cert_id} is already revoked")]
    AlreadyRevoked { cert_id: String },

    #[error("batch of {size} exceeds the maximum of {max}")]
    BatchTooLarge { size: usize, max: usize },

    #[error("batch must not be empty")]
    EmptyBatch {},

    #[error("cert_id must be 1..=128 characters")]
    InvalidCertId {},

    #[error("content_hash must be a 64-character lowercase hex sha256")]
    InvalidContentHash {},

    #[error("the issuer allowlist may not be left empty")]
    NoIssuersLeft {},
}
