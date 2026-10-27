#!/usr/bin/env python3
"""Synthetic raw backend characterization; no keys/stores/network or qualification.

Invoke with the explicitly selected Python. The identity-key vector demonstrates
why a successful ordinary positive witness cannot qualify strict parity.
"""
import json
import platform
from datetime import datetime, timezone
from importlib.metadata import version
from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.backends.openssl.backend import backend
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)


def outcome(public, signature, message):
    try:
        public.verify(signature, message)
        return "accepted"
    except (InvalidSignature, ValueError):
        return "refused"


def main():
    message = b"probe"
    identity_public = b"\x01" + bytes(31)
    identity_signature = b"\x01" + bytes(63)
    key = Ed25519PrivateKey.from_private_bytes(bytes([1]) * 32)
    signature = key.sign(message)
    scalar_order = 2**252 + 27742317777372353535851937790883648493
    noncanonical = signature[:32] + (
        int.from_bytes(signature[32:], "little") + scalar_order
    ).to_bytes(32, "little")
    cases = {
        "ordinary_positive": (signature, message),
        "signature_tamper": (bytes([signature[0] ^ 1]) + signature[1:], message),
        "message_substitution": (signature, b"other"),
        "noncanonical_scalar": (noncanonical, message),
        "invalid_R": (bytes([255]) * 32 + signature[32:], message),
        "signature_truncation": (signature[:-1], message),
    }
    print(json.dumps({
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
        "purpose": "raw backend characterization only; strict parity NOT qualified",
        "python": platform.python_version(),
        "cryptography": version("cryptography"),
        "backend": backend.openssl_version_text(),
        "identity_key_vector": {
            "public_key_hex": identity_public.hex(),
            "signature_hex": identity_signature.hex(),
            "message_hex": message.hex(),
            "pyca_backend_outcome": outcome(
                Ed25519PublicKey.from_public_bytes(identity_public),
                identity_signature, message
            ),
            "dalek_strict_required_outcome": "refused",
            "dalek_basis": "verify_strict rejects low-order public key and R; source inventory separately pins implementation",
        },
        "ordinary_synthetic_vectors": {
            name: {"signature_hex": sig.hex(), "message_hex": msg.hex(),
                   "pyca_backend_outcome": outcome(key.public_key(), sig, msg)}
            for name, (sig, msg) in cases.items()
        },
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
