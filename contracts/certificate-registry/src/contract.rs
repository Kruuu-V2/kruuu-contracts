use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult,
};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    CertificateInput, CertificateResponse, CertificatesResponse, ConfigResponse, ExecuteMsg,
    InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{Certificate, Config, BY_INSTITUTION, BY_RECIPIENT, CERTIFICATES, CONFIG, ISSUERS};

const CONTRACT_NAME: &str = "crates.io:kruuu-certificate-registry";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MAX_BATCH_SIZE: usize = 100;
const DEFAULT_QUERY_LIMIT: u32 = 30;
const MAX_QUERY_LIMIT: u32 = 100;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if msg.issuers.is_empty() {
        return Err(ContractError::NoIssuersLeft {});
    }

    let owner = deps.api.addr_validate(&msg.owner)?;
    CONFIG.save(
        deps.storage,
        &Config {
            owner: owner.clone(),
            total_issued: 0,
        },
    )?;

    for issuer in &msg.issuers {
        let addr = deps.api.addr_validate(issuer)?;
        ISSUERS.save(deps.storage, &addr, &())?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", owner)
        .add_attribute("issuers", msg.issuers.join(",")))
}

/// Only the CosmWasm admin (the multisig) can trigger this, and only for a
/// code id the multisig approved via a passed proposal. State is carried
/// over untouched; future versions hang their transforms off MigrateMsg.
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let stored = cw2::get_contract_version(deps.storage)?;
    if stored.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidMigration {
            expected: CONTRACT_NAME.to_string(),
            actual: stored.contract,
        });
    }
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", stored.version)
        .add_attribute("to_version", CONTRACT_VERSION))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Issue { cert } => execute_issue(deps, env, info, cert),
        ExecuteMsg::IssueBatch { certs } => execute_issue_batch(deps, env, info, certs),
        ExecuteMsg::Revoke { cert_id, reason } => execute_revoke(deps, env, info, cert_id, reason),
        ExecuteMsg::UpdateIssuers { add, remove } => execute_update_issuers(deps, info, add, remove),
        ExecuteMsg::UpdateOwner { new_owner } => execute_update_owner(deps, info, new_owner),
    }
}

fn ensure_issuer(deps: &DepsMut, sender: &Addr) -> Result<(), ContractError> {
    if !ISSUERS.has(deps.storage, sender) {
        return Err(ContractError::NotIssuer {});
    }
    Ok(())
}

fn ensure_owner(deps: &DepsMut, sender: &Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != *sender {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

fn validate_input(input: &CertificateInput) -> Result<(), ContractError> {
    if input.cert_id.is_empty() || input.cert_id.len() > 128 {
        return Err(ContractError::InvalidCertId {});
    }
    let hash_ok = input.content_hash.len() == 64
        && input
            .content_hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !hash_ok {
        return Err(ContractError::InvalidContentHash {});
    }
    Ok(())
}

fn store_certificate(
    deps: &mut DepsMut,
    env: &Env,
    issuer: &Addr,
    input: CertificateInput,
) -> Result<String, ContractError> {
    validate_input(&input)?;

    if CERTIFICATES.has(deps.storage, &input.cert_id) {
        return Err(ContractError::DuplicateCertificate {
            cert_id: input.cert_id,
        });
    }

    let certificate = Certificate {
        cert_id: input.cert_id.clone(),
        institution_id: input.institution_id,
        recipient_id: input.recipient_id,
        template_id: input.template_id,
        content_hash: input.content_hash,
        metadata_uri: input.metadata_uri,
        issued_at: input.issued_at.unwrap_or(env.block.time),
        issued_by: issuer.clone(),
        revoked: false,
        revoked_at: None,
        revoke_reason: None,
    };

    CERTIFICATES.save(deps.storage, &certificate.cert_id, &certificate)?;
    BY_RECIPIENT.save(
        deps.storage,
        (certificate.recipient_id, &certificate.cert_id),
        &(),
    )?;
    BY_INSTITUTION.save(
        deps.storage,
        (certificate.institution_id, &certificate.cert_id),
        &(),
    )?;

    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.total_issued += 1;
        Ok(config)
    })?;

    Ok(certificate.cert_id)
}

fn execute_issue(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cert: CertificateInput,
) -> Result<Response, ContractError> {
    ensure_issuer(&deps, &info.sender)?;

    let institution = cert.institution_id;
    let recipient = cert.recipient_id;
    let content_hash = cert.content_hash.clone();
    let cert_id = store_certificate(&mut deps, &env, &info.sender, cert)?;

    Ok(Response::new()
        .add_attribute("action", "issue")
        .add_attribute("cert_id", cert_id)
        .add_attribute("institution_id", institution.to_string())
        .add_attribute("recipient_id", recipient.to_string())
        .add_attribute("content_hash", content_hash))
}

fn execute_issue_batch(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    certs: Vec<CertificateInput>,
) -> Result<Response, ContractError> {
    ensure_issuer(&deps, &info.sender)?;

    if certs.is_empty() {
        return Err(ContractError::EmptyBatch {});
    }
    if certs.len() > MAX_BATCH_SIZE {
        return Err(ContractError::BatchTooLarge {
            size: certs.len(),
            max: MAX_BATCH_SIZE,
        });
    }

    let count = certs.len();
    let mut ids: Vec<String> = Vec::with_capacity(count);
    for cert in certs {
        ids.push(store_certificate(&mut deps, &env, &info.sender, cert)?);
    }

    Ok(Response::new()
        .add_attribute("action", "issue_batch")
        .add_attribute("count", count.to_string())
        .add_attribute("cert_ids", ids.join(",")))
}

fn execute_revoke(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cert_id: String,
    reason: Option<String>,
) -> Result<Response, ContractError> {
    ensure_issuer(&deps, &info.sender)?;

    let mut certificate = CERTIFICATES
        .may_load(deps.storage, &cert_id)?
        .ok_or(ContractError::CertificateNotFound {
            cert_id: cert_id.clone(),
        })?;

    if certificate.revoked {
        return Err(ContractError::AlreadyRevoked { cert_id });
    }

    certificate.revoked = true;
    certificate.revoked_at = Some(env.block.time);
    certificate.revoke_reason = reason;
    CERTIFICATES.save(deps.storage, &cert_id, &certificate)?;

    Ok(Response::new()
        .add_attribute("action", "revoke")
        .add_attribute("cert_id", cert_id))
}

fn execute_update_issuers(
    deps: DepsMut,
    info: MessageInfo,
    add: Vec<String>,
    remove: Vec<String>,
) -> Result<Response, ContractError> {
    ensure_owner(&deps, &info.sender)?;

    for issuer in &add {
        let addr = deps.api.addr_validate(issuer)?;
        ISSUERS.save(deps.storage, &addr, &())?;
    }
    for issuer in &remove {
        let addr = deps.api.addr_validate(issuer)?;
        ISSUERS.remove(deps.storage, &addr);
    }

    // A registry with no issuer can never anchor again without owner action;
    // refuse to end up there by accident.
    let any_left = ISSUERS
        .keys(deps.storage, None, None, Order::Ascending)
        .next()
        .is_some();
    if !any_left {
        return Err(ContractError::NoIssuersLeft {});
    }

    Ok(Response::new()
        .add_attribute("action", "update_issuers")
        .add_attribute("added", add.join(","))
        .add_attribute("removed", remove.join(",")))
}

fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    ensure_owner(&deps, &info.sender)?;

    let new_owner = deps.api.addr_validate(&new_owner)?;
    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.owner = new_owner.clone();
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_owner")
        .add_attribute("new_owner", new_owner))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Certificate { cert_id } => to_json_binary(&query_certificate(deps, cert_id)?),
        QueryMsg::CertificatesByRecipient {
            recipient_id,
            start_after,
            limit,
        } => to_json_binary(&query_by_index(
            deps,
            IndexKind::Recipient(recipient_id),
            start_after,
            limit,
        )?),
        QueryMsg::CertificatesByInstitution {
            institution_id,
            start_after,
            limit,
        } => to_json_binary(&query_by_index(
            deps,
            IndexKind::Institution(institution_id),
            start_after,
            limit,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let issuers = ISSUERS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|key| key.map(|addr| addr.to_string()))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        issuers,
        total_issued: config.total_issued,
    })
}

fn query_certificate(deps: Deps, cert_id: String) -> StdResult<CertificateResponse> {
    let certificate = CERTIFICATES.load(deps.storage, &cert_id)?;
    Ok(CertificateResponse { certificate })
}

enum IndexKind {
    Recipient(u64),
    Institution(u64),
}

fn query_by_index(
    deps: Deps,
    kind: IndexKind,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<CertificatesResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.as_deref().map(Bound::exclusive);

    let ids: Vec<String> = match kind {
        IndexKind::Recipient(id) => BY_RECIPIENT
            .prefix(id)
            .keys(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?,
        IndexKind::Institution(id) => BY_INSTITUTION
            .prefix(id)
            .keys(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?,
    };

    let certificates = ids
        .iter()
        .map(|cert_id| CERTIFICATES.load(deps.storage, cert_id))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CertificatesResponse { certificates })
}
