use cosmwasm_std::{Addr, Timestamp};
use cw_multi_test::{App, ContractWrapper, Executor};

use crate::contract::{execute, instantiate, migrate, query, MAX_BATCH_SIZE};
use crate::error::ContractError;
use crate::msg::{
    CertificateInput, CertificateResponse, CertificatesResponse, ConfigResponse, ExecuteMsg,
    InstantiateMsg, MigrateMsg, QueryMsg,
};

struct TestEnv {
    app: App,
    contract: Addr,
    code_id: u64,
    owner: Addr,
    issuer: Addr,
    rando: Addr,
}

fn hash(seed: u8) -> String {
    format!("{:064x}", seed as u128)
}

fn cert(id: &str, institution: u64, recipient: u64) -> CertificateInput {
    CertificateInput {
        cert_id: id.to_string(),
        institution_id: institution,
        recipient_id: recipient,
        template_id: 1,
        content_hash: hash(7),
        metadata_uri: Some(format!("ipfs://meta/{id}")),
        issued_at: None,
    }
}

fn setup() -> TestEnv {
    let mut app = App::default();
    let owner = app.api().addr_make("owner-multisig");
    let issuer = app.api().addr_make("relayer");
    let rando = app.api().addr_make("someone-else");

    let code = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
    let code_id = app.store_code(Box::new(code));

    let contract = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &InstantiateMsg {
                owner: owner.to_string(),
                issuers: vec![issuer.to_string()],
            },
            &[],
            "kruuu-certificate-registry",
            Some(owner.to_string()),
        )
        .unwrap();

    TestEnv {
        app,
        contract,
        code_id,
        owner,
        issuer,
        rando,
    }
}

fn get_cert(env: &TestEnv, cert_id: &str) -> CertificateResponse {
    env.app
        .wrap()
        .query_wasm_smart(
            env.contract.clone(),
            &QueryMsg::Certificate {
                cert_id: cert_id.to_string(),
            },
        )
        .unwrap()
}

#[test]
fn issue_and_query_roundtrip() {
    let mut env = setup();

    env.app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 100, 200),
            },
            &[],
        )
        .unwrap();

    let res = get_cert(&env, "cert:1");
    assert_eq!(res.certificate.institution_id, 100);
    assert_eq!(res.certificate.recipient_id, 200);
    assert_eq!(res.certificate.issued_by, env.issuer);
    assert!(!res.certificate.revoked);
    // issued_at defaulted to block time
    assert!(res.certificate.issued_at > Timestamp::from_seconds(0));

    let config: ConfigResponse = env
        .app
        .wrap()
        .query_wasm_smart(env.contract.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.total_issued, 1);
}

#[test]
fn non_issuer_cannot_issue() {
    let mut env = setup();

    let err = env
        .app
        .execute_contract(
            env.rando.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 100, 200),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::NotIssuer {}
    );

    // even the owner is not an issuer by default — governance and issuance
    // are deliberately separate powers
    let err = env
        .app
        .execute_contract(
            env.owner.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 100, 200),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::NotIssuer {}
    );
}

#[test]
fn duplicate_cert_id_is_rejected() {
    let mut env = setup();

    env.app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 100, 200),
            },
            &[],
        )
        .unwrap();

    let err = env
        .app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 101, 201),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::DuplicateCertificate {
            cert_id: "cert:1".to_string()
        }
    );
}

#[test]
fn input_validation() {
    let mut env = setup();

    let mut bad_hash = cert("cert:1", 1, 2);
    bad_hash.content_hash = "not-a-hash".to_string();
    let err = env
        .app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue { cert: bad_hash },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidContentHash {}
    );

    let mut bad_id = cert("", 1, 2);
    bad_id.cert_id = String::new();
    let err = env
        .app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue { cert: bad_id },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::InvalidCertId {}
    );
}

#[test]
fn batch_issue_with_backfill_dates() {
    let mut env = setup();

    let mut old = cert("cert:old", 1, 2);
    old.issued_at = Some(Timestamp::from_seconds(1_600_000_000));

    env.app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::IssueBatch {
                certs: vec![old, cert("cert:new", 1, 3)],
            },
            &[],
        )
        .unwrap();

    let res = get_cert(&env, "cert:old");
    assert_eq!(res.certificate.issued_at, Timestamp::from_seconds(1_600_000_000));

    let config: ConfigResponse = env
        .app
        .wrap()
        .query_wasm_smart(env.contract.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.total_issued, 2);
}

#[test]
fn batch_limits() {
    let mut env = setup();

    let err = env
        .app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::IssueBatch { certs: vec![] },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::EmptyBatch {}
    );

    let too_many: Vec<_> = (0..=MAX_BATCH_SIZE)
        .map(|i| cert(&format!("cert:{i}"), 1, 1))
        .collect();
    let err = env
        .app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::IssueBatch { certs: too_many },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::BatchTooLarge {
            size: MAX_BATCH_SIZE + 1,
            max: MAX_BATCH_SIZE
        }
    );
}

#[test]
fn batch_is_atomic() {
    let mut env = setup();

    // second entry duplicates the first — the whole batch must fail
    let err = env
        .app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::IssueBatch {
                certs: vec![cert("cert:1", 1, 2), cert("cert:1", 1, 3)],
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::DuplicateCertificate {
            cert_id: "cert:1".to_string()
        }
    );

    let config: ConfigResponse = env
        .app
        .wrap()
        .query_wasm_smart(env.contract.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.total_issued, 0);
}

#[test]
fn revoke_keeps_record() {
    let mut env = setup();

    env.app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 100, 200),
            },
            &[],
        )
        .unwrap();

    env.app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Revoke {
                cert_id: "cert:1".to_string(),
                reason: Some("issued in error".to_string()),
            },
            &[],
        )
        .unwrap();

    let res = get_cert(&env, "cert:1");
    assert!(res.certificate.revoked);
    assert_eq!(
        res.certificate.revoke_reason,
        Some("issued in error".to_string())
    );

    // double-revoke refused
    let err = env
        .app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Revoke {
                cert_id: "cert:1".to_string(),
                reason: None,
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::AlreadyRevoked {
            cert_id: "cert:1".to_string()
        }
    );
}

#[test]
fn recipient_and_institution_indexes() {
    let mut env = setup();

    env.app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::IssueBatch {
                certs: vec![
                    cert("cert:a", 10, 500),
                    cert("cert:b", 10, 500),
                    cert("cert:c", 11, 500),
                    cert("cert:d", 10, 600),
                ],
            },
            &[],
        )
        .unwrap();

    let by_recipient: CertificatesResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            env.contract.clone(),
            &QueryMsg::CertificatesByRecipient {
                recipient_id: 500,
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(by_recipient.certificates.len(), 3);

    let by_institution: CertificatesResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            env.contract.clone(),
            &QueryMsg::CertificatesByInstitution {
                institution_id: 10,
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(by_institution.certificates.len(), 3);

    // pagination: page of 2, then the rest
    let page1: CertificatesResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            env.contract.clone(),
            &QueryMsg::CertificatesByRecipient {
                recipient_id: 500,
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap();
    assert_eq!(page1.certificates.len(), 2);

    let page2: CertificatesResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            env.contract.clone(),
            &QueryMsg::CertificatesByRecipient {
                recipient_id: 500,
                start_after: Some(page1.certificates[1].cert_id.clone()),
                limit: Some(2),
            },
        )
        .unwrap();
    assert_eq!(page2.certificates.len(), 1);
}

#[test]
fn owner_governs_issuers() {
    let mut env = setup();
    let new_issuer = env.app.api().addr_make("new-relayer");

    // rando cannot touch the allowlist
    let err = env
        .app
        .execute_contract(
            env.rando.clone(),
            env.contract.clone(),
            &ExecuteMsg::UpdateIssuers {
                add: vec![new_issuer.to_string()],
                remove: vec![],
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );

    // owner rotates the relayer
    env.app
        .execute_contract(
            env.owner.clone(),
            env.contract.clone(),
            &ExecuteMsg::UpdateIssuers {
                add: vec![new_issuer.to_string()],
                remove: vec![env.issuer.to_string()],
            },
            &[],
        )
        .unwrap();

    // old key can no longer issue, new one can
    let err = env
        .app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 1, 2),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::NotIssuer {}
    );

    env.app
        .execute_contract(
            new_issuer,
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 1, 2),
            },
            &[],
        )
        .unwrap();

    // cannot empty the allowlist
    let config: ConfigResponse = env
        .app
        .wrap()
        .query_wasm_smart(env.contract.clone(), &QueryMsg::Config {})
        .unwrap();
    let err = env
        .app
        .execute_contract(
            env.owner.clone(),
            env.contract.clone(),
            &ExecuteMsg::UpdateIssuers {
                add: vec![],
                remove: config.issuers,
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::NoIssuersLeft {}
    );
}

#[test]
fn ownership_transfer() {
    let mut env = setup();
    let new_owner = env.app.api().addr_make("new-multisig");

    let err = env
        .app
        .execute_contract(
            env.rando.clone(),
            env.contract.clone(),
            &ExecuteMsg::UpdateOwner {
                new_owner: new_owner.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );

    env.app
        .execute_contract(
            env.owner.clone(),
            env.contract.clone(),
            &ExecuteMsg::UpdateOwner {
                new_owner: new_owner.to_string(),
            },
            &[],
        )
        .unwrap();

    let config: ConfigResponse = env
        .app
        .wrap()
        .query_wasm_smart(env.contract.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.owner, new_owner.to_string());

    // the old owner is out
    let err = env
        .app
        .execute_contract(
            env.owner.clone(),
            env.contract.clone(),
            &ExecuteMsg::UpdateIssuers {
                add: vec![],
                remove: vec![],
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );
}

#[test]
fn admin_can_migrate_and_state_survives() {
    let mut env = setup();

    env.app
        .execute_contract(
            env.issuer.clone(),
            env.contract.clone(),
            &ExecuteMsg::Issue {
                cert: cert("cert:1", 10, 20),
            },
            &[],
        )
        .unwrap();

    env.app
        .migrate_contract(
            env.owner.clone(),
            env.contract.clone(),
            &MigrateMsg {},
            env.code_id,
        )
        .unwrap();

    let stored = get_cert(&env, "cert:1").certificate;
    assert_eq!(stored.cert_id, "cert:1");
    let config: ConfigResponse = env
        .app
        .wrap()
        .query_wasm_smart(env.contract.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.total_issued, 1);
    assert_eq!(config.owner, env.owner.to_string());
    assert_eq!(config.issuers, vec![env.issuer.to_string()]);
}

#[test]
fn non_admin_cannot_migrate() {
    let mut env = setup();

    for sender in [env.rando.clone(), env.issuer.clone()] {
        env.app
            .migrate_contract(sender, env.contract.clone(), &MigrateMsg {}, env.code_id)
            .unwrap_err();
    }
}

#[test]
fn migrate_rejects_foreign_contract_state() {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    let mut deps = mock_dependencies();
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:some-other-contract", "9.9.9")
        .unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap_err();
    assert!(matches!(err, ContractError::InvalidMigration { .. }));
}
