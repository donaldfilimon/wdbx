//! HMAC admission envelopes for typed key/value versions.
//!
//! Each stored value is `{"schema":"wdbx-typed-kv-v1","admission":{…},"value":…}`.
//! The admission MAC is HMAC-SHA256 under the host key over a fixed-order claim
//! set binding namespace, writer, typed sequence, nonce, the granted principal,
//! the policy/consent epoch, and the exact key and value.

use crate::error::TypedError;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;

pub(crate) const ENVELOPE_SCHEMA: &str = "wdbx-typed-kv-v1";
const CLAIM_DOMAIN: &str = "wdbx-typed-admission-v1";

pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn hmac_sha256(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    let mut inner_pad = [0x36_u8; 64];
    let mut outer_pad = [0x5c_u8; 64];
    for (index, byte) in key.iter().enumerate() {
        inner_pad[index] ^= byte;
        outer_pad[index] ^= byte;
    }
    let inner = Sha256::new()
        .chain_update(inner_pad)
        .chain_update(message)
        .finalize();
    Sha256::new()
        .chain_update(outer_pad)
        .chain_update(inner)
        .finalize()
        .into()
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
            == 0
}

/// The admitted facts a version carries, as lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Admission {
    pub(crate) writer: String,
    pub(crate) sequence: u64,
    pub(crate) nonce: String,
    pub(crate) principal: String,
    pub(crate) policy: String,
    pub(crate) consent: String,
    pub(crate) mac: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema: String,
    admission: Admission,
    value: String,
}

#[derive(Serialize)]
struct Claims<'a> {
    domain: &'static str,
    namespace: &'a str,
    writer: &'a str,
    sequence: u64,
    nonce: &'a str,
    principal: &'a str,
    role: &'static str,
    policy: &'a str,
    consent: &'a str,
    key: &'a str,
    value: &'a str,
}

fn mac(
    host_key: &[u8; 32],
    namespace: &str,
    admission: &Admission,
    key: &str,
    value: &str,
) -> Result<String, TypedError> {
    let claims = Claims {
        domain: CLAIM_DOMAIN,
        namespace,
        writer: &admission.writer,
        sequence: admission.sequence,
        nonce: &admission.nonce,
        principal: &admission.principal,
        role: "service",
        policy: &admission.policy,
        consent: &admission.consent,
        key,
        value,
    };
    let message = serde_json::to_vec(&claims)
        .map_err(|error| TypedError::new("Internal", error.to_string()))?;
    Ok(hex(&hmac_sha256(host_key, &message)))
}

/// Seal one value: `admission.mac` is ignored on input and computed here.
pub(crate) fn seal(
    host_key: &[u8; 32],
    namespace: &str,
    mut admission: Admission,
    key: &str,
    value: &str,
) -> Result<String, TypedError> {
    admission.mac = mac(host_key, namespace, &admission, key, value)?;
    serde_json::to_string(&Envelope {
        schema: ENVELOPE_SCHEMA.into(),
        admission,
        value: value.into(),
    })
    .map_err(|error| TypedError::new("Internal", error.to_string()))
}

/// Authenticate one stored version and return its admission and inner value.
pub(crate) fn open(
    host_key: &[u8; 32],
    namespace: &str,
    key: &str,
    stored: &str,
) -> Result<(Admission, String), TypedError> {
    let envelope: Envelope = serde_json::from_str(stored).map_err(|_| {
        TypedError::new(
            "AdmissionInvalid",
            format!("key {key:?} holds a value that is not a typed envelope"),
        )
    })?;
    if envelope.schema != ENVELOPE_SCHEMA {
        return Err(TypedError::new(
            "AdmissionInvalid",
            format!("key {key:?} has an unsupported envelope schema"),
        ));
    }
    let expected = mac(
        host_key,
        namespace,
        &envelope.admission,
        key,
        &envelope.value,
    )?;
    if !constant_time_equal(expected.as_bytes(), envelope.admission.mac.as_bytes()) {
        return Err(TypedError::new(
            "AdmissionInvalid",
            format!("key {key:?} failed host admission authentication"),
        ));
    }
    Ok((envelope.admission, envelope.value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_rfc_4231_case_2_shape_with_a_32_byte_key() {
        // RFC 4231 test case 2 uses a short key; zero-padding it to 32 bytes is
        // the same HMAC input block, so the published digest must match.
        let mut key = [0_u8; 32];
        key[..4].copy_from_slice(b"Jefe");
        assert_eq!(
            hex(&hmac_sha256(&key, b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn sealed_values_open_only_under_the_same_key_namespace_and_key() {
        let host = [7_u8; 32];
        let admission = Admission {
            writer: "06".repeat(16),
            sequence: 1,
            nonce: "01".repeat(32),
            principal: "05".repeat(32),
            policy: "03".repeat(32),
            consent: "04".repeat(32),
            mac: String::new(),
        };
        let stored = seal(&host, "ns", admission.clone(), "invoice", "sealed").unwrap();
        let (opened, value) = open(&host, "ns", "invoice", &stored).unwrap();
        assert_eq!(value, "sealed");
        assert_eq!(opened.sequence, 1);
        assert!(open(&[8_u8; 32], "ns", "invoice", &stored).is_err());
        assert!(open(&host, "other", "invoice", &stored).is_err());
        assert!(open(&host, "ns", "renamed", &stored).is_err());
        let tampered = stored.replace("\"sealed\"", "\"forged\"");
        assert!(open(&host, "ns", "invoice", &tampered).is_err());
    }
}
