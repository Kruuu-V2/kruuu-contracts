use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Timestamp;

use crate::state::Certificate;

#[cw_serde]
pub struct InstantiateMsg {
    /// Governance address — intended to be the cw3-flex multisig. Controls the
    /// issuer allowlist and ownership transfer. CosmWasm-level migration admin
    /// is set separately at instantiation time.
    pub owner: String,
    /// Addresses allowed to issue and revoke certificates (the backend relayer).
    pub issuers: Vec<String>,
}

/// Payload for the CosmWasm-admin migrate call (the multisig). Carries no
/// fields today; a future code version adds fields and state transforms here.
#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct CertificateInput {
    /// Globally unique id, chosen by the issuer. Kruuu uses
    /// "cert:{certeficateToUsers.id}" so a DB row maps 1:1 to a chain record.
    pub cert_id: String,
    /// Kruuu institution user id that issued the certificate.
    pub institution_id: u64,
    /// Kruuu talent user id that received it. Opaque on purpose: no wallet
    /// address, nothing derivable from an email.
    pub recipient_id: u64,
    /// Kruuu certificate template id (the `certifications` row).
    pub template_id: u64,
    /// sha256 hex of the canonical certificate metadata JSON.
    pub content_hash: String,
    /// Optional pointer to the pinned metadata (IPFS gateway URL or ipfs:// URI).
    pub metadata_uri: Option<String>,
    /// Original issuance time — lets the backfill preserve historical dates.
    /// Defaults to the block time when omitted.
    pub issued_at: Option<Timestamp>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Anchor one certificate. Issuer-only. Fails on duplicate cert_id.
    Issue { cert: CertificateInput },
    /// Anchor up to MAX_BATCH_SIZE certificates in one tx (backfill).
    /// Issuer-only. All-or-nothing.
    IssueBatch { certs: Vec<CertificateInput> },
    /// Mark a certificate revoked. The record stays queryable. Issuer-only.
    Revoke {
        cert_id: String,
        reason: Option<String>,
    },
    /// Owner-only: change the issuer allowlist.
    UpdateIssuers {
        add: Vec<String>,
        remove: Vec<String>,
    },
    /// Owner-only: hand governance to a new owner (e.g. a new multisig).
    UpdateOwner { new_owner: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(CertificateResponse)]
    Certificate { cert_id: String },
    #[returns(CertificatesResponse)]
    CertificatesByRecipient {
        recipient_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(CertificatesResponse)]
    CertificatesByInstitution {
        institution_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub issuers: Vec<String>,
    pub total_issued: u64,
}

#[cw_serde]
pub struct CertificateResponse {
    pub certificate: Certificate,
}

#[cw_serde]
pub struct CertificatesResponse {
    pub certificates: Vec<Certificate>,
}
