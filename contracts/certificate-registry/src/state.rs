use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub total_issued: u64,
}

#[cw_serde]
pub struct Certificate {
    pub cert_id: String,
    pub institution_id: u64,
    pub recipient_id: u64,
    pub template_id: u64,
    pub content_hash: String,
    pub metadata_uri: Option<String>,
    pub issued_at: Timestamp,
    pub issued_by: Addr,
    pub revoked: bool,
    pub revoked_at: Option<Timestamp>,
    pub revoke_reason: Option<String>,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Issuer allowlist. Value is a unit — presence is the permission.
pub const ISSUERS: Map<&Addr, ()> = Map::new("issuers");

/// Primary store: cert_id -> certificate.
pub const CERTIFICATES: Map<&str, Certificate> = Map::new("certificates");

/// Secondary indexes for the two list queries. Value is a unit; the key
/// composite is (id, cert_id) so a prefix scan lists one owner's certs.
pub const BY_RECIPIENT: Map<(u64, &str), ()> = Map::new("by_recipient");
pub const BY_INSTITUTION: Map<(u64, &str), ()> = Map::new("by_institution");
