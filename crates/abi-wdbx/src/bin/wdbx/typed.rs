//! `wdbx typed inspect|commit`: one configured service writer over a v2 root.
//!
//! The typed frontier is `{configured writer: n}` where `n` is the number of
//! committed v2 transactions in the root; every one of them is a typed commit,
//! made under the exclusive typed lock after checking that the caller observed
//! exactly that frontier. V2 writer identities underneath are per-process and
//! not part of this contract.

use crate::admission::{self, Admission, hex};
use crate::error::TypedError;
use crate::files;
use abi_wdbx::v2::{ObjectSecurity, V2Snapshot, V2TemporalKind};
use abi_wdbx::{V2Mutation, V2Store, read_v2_root};
use serde::Deserialize;
use serde_json::json;
use std::path::Path;

const CONFIG_SCHEMA: &str = "wdbx-typed-host-config-v1";
const AUTH_SCHEMA: &str = "wdbx-typed-host-authorization-v1";
const TRANSACTION_SCHEMA: &str = "wdbx-typed-transaction-input-v1";
const INSPECTION_SCHEMA: &str = "wdbx-typed-inspection-v1";
const RECEIPT_SCHEMA: &str = "wdbx-typed-commit-receipt-v1";
const MAX_JSON_BYTES: u64 = 1024 * 1024;
const MAX_MUTATIONS: usize = 16;
const MAX_VALUE_BYTES: usize = 64 * 1024;

#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Grant {
    principal: [u8; 32],
    role: String,
    writer: [u8; 16],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Epoch {
    policy: [u8; 32],
    consent: [u8; 32],
    valid_from: i64,
    valid_until: i64,
    grants: Vec<Grant>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema: String,
    namespace: [u8; 32],
    current: Epoch,
    history: Vec<Epoch>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authorization {
    allowed: bool,
    writer_allowed: bool,
    principal: [u8; 32],
    role: String,
    namespace: [u8; 32],
    policy: [u8; 32],
    consent: [u8; 32],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthInput {
    schema: String,
    authorization: Authorization,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Head {
    writer: [u8; 16],
    sequence: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Kv {
    key: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationInput {
    kv: Kv,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transaction {
    writer: [u8; 16],
    sequence: u64,
    nonce: [u8; 32],
    observed: Vec<Head>,
    mutations: Vec<MutationInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionInput {
    schema: String,
    transaction: Transaction,
}

pub(crate) struct Paths<'a> {
    dir: &'a Path,
    config: &'a Path,
    host_key: &'a Path,
}

impl<'a> Paths<'a> {
    pub(crate) fn new(
        dir: &'a str,
        config: &'a str,
        host_key: &'a str,
    ) -> Result<Self, TypedError> {
        Ok(Self {
            dir: files::absolute(dir)?,
            config: files::absolute(config)?,
            host_key: files::absolute(host_key)?,
        })
    }
}

/// Validated host configuration and key for one operation.
struct Host {
    config: Config,
    key: [u8; 32],
    namespace: String,
}

impl Host {
    fn grant(&self) -> &Grant {
        &self.config.current.grants[0]
    }

    /// A recorded policy/consent pair is admissible if any configured epoch names it.
    fn knows_epoch(&self, policy: &str, consent: &str) -> bool {
        std::iter::once(&self.config.current)
            .chain(&self.config.history)
            .any(|epoch| hex(&epoch.policy) == policy && hex(&epoch.consent) == consent)
    }
}

fn parse<T: for<'de> Deserialize<'de>>(bytes: &[u8], what: &str) -> Result<T, TypedError> {
    serde_json::from_slice(bytes)
        .map_err(|error| TypedError::new("InvalidInput", format!("{what}: {error}")))
}

fn load_host(paths: &Paths<'_>) -> Result<Host, TypedError> {
    files::private_directory(paths.dir)?;
    let config: Config = parse(
        &files::read_private_file(paths.config, MAX_JSON_BYTES)?,
        "host configuration",
    )?;
    if config.schema != CONFIG_SCHEMA {
        return Err(TypedError::new(
            "InvalidConfig",
            "unsupported host configuration schema",
        ));
    }
    let [grant] = config.current.grants.as_slice() else {
        return Err(TypedError::new(
            "InvalidConfig",
            "the current epoch must name exactly one grant",
        ));
    };
    if grant.role != "service" {
        return Err(TypedError::new(
            "InvalidConfig",
            "the grant role must be service",
        ));
    }
    for epoch in std::iter::once(&config.current).chain(&config.history) {
        if epoch.valid_from >= epoch.valid_until {
            return Err(TypedError::new("InvalidConfig", "epoch bounds are empty"));
        }
        if epoch.grants.as_slice() != std::slice::from_ref(grant) {
            return Err(TypedError::new(
                "InvalidConfig",
                "every epoch must retain the one stable writer grant",
            ));
        }
    }
    let key: [u8; 32] = files::read_private_file(paths.host_key, 32)?
        .try_into()
        .map_err(|_| TypedError::new("InvalidConfig", "the host key must be exactly 32 bytes"))?;
    let namespace = hex(&config.namespace);
    Ok(Host {
        config,
        key,
        namespace,
    })
}

/// The authenticated typed state of one root.
struct State {
    sequence: u64,
    rows: Vec<serde_json::Value>,
}

fn read_snapshot(dir: &Path) -> Result<Option<V2Snapshot>, TypedError> {
    if !dir.join("VERSION").exists() {
        if dir.join("journals").exists() {
            return Err(TypedError::new(
                "StoreCorrupt",
                "journals exist without a v2 VERSION marker",
            ));
        }
        return Ok(None);
    }
    read_v2_root(dir, &ObjectSecurity::plaintext())
        .map(Some)
        .map_err(|error| TypedError::new("StoreUnavailable", error.to_string()))
}

fn verified_state(host: &Host, dir: &Path) -> Result<State, TypedError> {
    let Some(snapshot) = read_snapshot(dir)? else {
        return Ok(State {
            sequence: 0,
            rows: Vec::new(),
        });
    };
    if snapshot.vector_count() > 0
        || snapshot.spatial_count() > 0
        || snapshot.audit_count() > 0
        || snapshot.temporal_count(V2TemporalKind::Node) > 0
        || snapshot.temporal_count(V2TemporalKind::Edge) > 0
    {
        return Err(TypedError::new(
            "StoreCorrupt",
            "the store holds records outside the typed key/value contract",
        ));
    }
    let committed = u64::try_from(snapshot.committed_transactions())
        .map_err(|_| TypedError::new("StoreCorrupt", "transaction count overflow"))?;
    let writer = hex(&host.grant().writer);
    let principal = hex(&host.grant().principal);
    let mut highest = 0;
    let mut rows = Vec::new();
    for key in snapshot.kv_keys() {
        // Authenticate history before using the maximal-version projection:
        // a valid replacement must not conceal an invalid earlier envelope.
        for version in snapshot.kv_versions(key).unwrap_or_default() {
            let (admitted, _) = admission::open(&host.key, &host.namespace, key, &version.value)?;
            if admitted.writer != writer
                || admitted.principal != principal
                || !host.knows_epoch(&admitted.policy, &admitted.consent)
            {
                return Err(TypedError::new(
                    "AdmissionInvalid",
                    format!("key {key:?} was admitted for another writer or epoch"),
                ));
            }
            highest = highest.max(admitted.sequence);
        }
        let Some(set) = snapshot.get(key) else {
            continue;
        };
        let mut value = None;
        let mut ids = Vec::new();
        for version in std::iter::once(&set.preferred).chain(&set.conflicts) {
            let (_, inner) = admission::open(&host.key, &host.namespace, key, &version.value)?;
            ids.push(version.version_id.to_string());
            value.get_or_insert(inner);
        }
        rows.push(json!({
            "current": true,
            "conflict_ids": ids,
            "mutation": { "kv": { "key": key, "value": value } },
        }));
    }
    if highest != committed {
        return Err(TypedError::new(
            "StoreCorrupt",
            "typed sequence does not match the committed transactions",
        ));
    }
    Ok(State {
        sequence: committed,
        rows,
    })
}

fn frontier(host: &Host, sequence: u64) -> serde_json::Value {
    if sequence == 0 {
        json!([])
    } else {
        json!([{ "writer_id": hex(&host.grant().writer), "sequence": sequence }])
    }
}

pub(crate) fn inspect(paths: &Paths<'_>) -> Result<String, TypedError> {
    let host = load_host(paths)?;
    let state = verified_state(&host, paths.dir)?;
    Ok(json!({
        "schema": INSPECTION_SCHEMA,
        "namespace": host.namespace,
        "frontier": frontier(&host, state.sequence),
        "rows": state.rows,
    })
    .to_string())
}

fn now_ms() -> Result<i64, TypedError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .ok_or_else(|| TypedError::new("Internal", "system clock is before the Unix epoch"))
}

fn admit(host: &Host, auth: &AuthInput, transaction: &Transaction) -> Result<(), TypedError> {
    let current = &host.config.current;
    let grant = host.grant();
    let authorization = &auth.authorization;
    if auth.schema != AUTH_SCHEMA
        || !authorization.allowed
        || !authorization.writer_allowed
        || authorization.role != "service"
        || authorization.principal != grant.principal
        || authorization.namespace != host.config.namespace
        || authorization.policy != current.policy
        || authorization.consent != current.consent
    {
        return Err(TypedError::new(
            "AdmissionDenied",
            "the authorization does not match the current host epoch",
        ));
    }
    let now = now_ms()?;
    if now < current.valid_from || now >= current.valid_until {
        return Err(TypedError::new(
            "AdmissionDenied",
            "the current host epoch is not valid now",
        ));
    }
    if transaction.writer != grant.writer {
        return Err(TypedError::new(
            "AdmissionDenied",
            "the transaction writer is not the granted writer",
        ));
    }
    if transaction.mutations.is_empty() || transaction.mutations.len() > MAX_MUTATIONS {
        return Err(TypedError::new(
            "InvalidInput",
            format!("a transaction carries 1..={MAX_MUTATIONS} mutations"),
        ));
    }
    for mutation in &transaction.mutations {
        if mutation.kv.key.is_empty() || mutation.kv.value.len() > MAX_VALUE_BYTES {
            return Err(TypedError::new(
                "InvalidInput",
                format!("keys are non-empty and values at most {MAX_VALUE_BYTES} bytes"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn commit(
    paths: &Paths<'_>,
    auth_path: &Path,
    transaction_path: &Path,
) -> Result<String, TypedError> {
    let host = load_host(paths)?;
    let auth: AuthInput = parse(
        &files::read_private_file(auth_path, MAX_JSON_BYTES)?,
        "authorization",
    )?;
    let input: TransactionInput = parse(
        &files::read_private_file(transaction_path, MAX_JSON_BYTES)?,
        "transaction",
    )?;
    if input.schema != TRANSACTION_SCHEMA {
        return Err(TypedError::new(
            "InvalidInput",
            "unsupported transaction schema",
        ));
    }
    let transaction = input.transaction;
    admit(&host, &auth, &transaction)?;
    let _lock = files::acquire_lock(paths.dir)?;
    let observed = verified_state(&host, paths.dir)?.sequence;
    let expected: Vec<Head> = if observed == 0 {
        Vec::new()
    } else {
        vec![Head {
            writer: host.grant().writer,
            sequence: observed,
        }]
    };
    if transaction.sequence != observed + 1 || transaction.observed != expected {
        return Err(TypedError::stale(format!(
            "the store frontier is at sequence {observed}"
        )));
    }
    let admission = Admission {
        writer: hex(&transaction.writer),
        sequence: transaction.sequence,
        nonce: hex(&transaction.nonce),
        principal: hex(&host.grant().principal),
        policy: hex(&host.config.current.policy),
        consent: hex(&host.config.current.consent),
        mac: String::new(),
    };
    let mutations = transaction
        .mutations
        .iter()
        .map(|mutation| {
            admission::seal(
                &host.key,
                &host.namespace,
                admission.clone(),
                &mutation.kv.key,
                &mutation.kv.value,
            )
            .map(|value| V2Mutation::PutKv {
                key: mutation.kv.key.clone(),
                value,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    {
        let mut store = V2Store::open_with_security(paths.dir, ObjectSecurity::plaintext())
            .map_err(|error| TypedError::new("StoreUnavailable", error.to_string()))?;
        store
            .commit(mutations)
            .map_err(|error| TypedError::new("StoreUnavailable", error.to_string()))?;
    }
    let published = verified_state(&host, paths.dir)?.sequence;
    if published != transaction.sequence {
        return Err(TypedError::new(
            "StoreCorrupt",
            "the committed sequence was not published",
        ));
    }
    Ok(json!({
        "schema": RECEIPT_SCHEMA,
        "writer_id": hex(&transaction.writer),
        "sequence": published,
    })
    .to_string())
}
