#!/usr/bin/env python3
"""Second-language *encoding* witness for the private `cose-sign1-p1` profile.

This is an independent, standard-library-only Python reimplementation of the
container **encoding** and of the RFC 9052 `Sig_structure` construction in
`crates/abi-wdbx/src/v3/episode/cose/`. It exists to give the Rust encoder a
cross-language witness: the same container bytes must parse to the same
protected header, key id, algorithm, payload and `Sig_structure` bytes here and
in Rust, and the same malformed containers must be refused with the same error
names.

Scope, stated so nobody infers more from the file name:

- It reimplements the bounded container reader, the bounded protected-header
  reader, and the `Sig_structure` builder.
- It does **not** verify signatures. There is no Ed25519 implementation here and
  none is wanted: this environment has no Python cryptography library, and this
  repository's design guidance forbids hand-rolling cryptography in Python. A
  successful parse says nothing whatsoever about whether the 64 signature bytes
  are a valid Ed25519 signature. Cryptographic verification stays Rust-side.
  Every response this script emits carries `"scope": "encoding-agreement-only"`.
- It is not a COSE library, not a general COSE reader, and not a ratification
  of the profile. The profile it witnesses is private, crate-internal and
  unratified.
- It does not implement admission, authorization, trust, or key custody.

Commands:

    parse-json-lines        bounded JSON-lines encoding witness; reads
                            {"container_hex": "<lowercase hex>"} per line and
                            writes one JSON object per line:
                            {"ok": true, "scope": "encoding-agreement-only",
                             "protected_hex": ..., "sig_structure_hex": ...,
                             "kid": ..., "alg": -19, "payload_hex": ...,
                             "signature_bytes_hex": ...}
                            or {"ok": false, "scope": "encoding-agreement-only",
                             "error": "<RustErrorName>"}
    inspect-hex <hex>       parse one container given as hex and print the same
                            fields as one JSON object
    verify-goldens [DIR]    rebuild the vectors described in DIR/README.md and
                            compare them with DIR/*.hex, DIR/*.sha256 and
                            DIR/alg.txt
                            (default DIR: crates/abi-wdbx/tests/golden/cose-sign1-p1)

Exit status is 0 when every check passed and 1 otherwise. The script never
prints input values in error messages, mirroring the content-free Rust errors.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys

# --- profile constants, mirrored from src/v3/episode/cose/profile.rs ---------

PROFILE_NAME = "cose-sign1-p1"

#: CBOR tag number that identifies a `COSE_Sign1` object.
COSE_SIGN1_TAG = 18

#: COSE algorithm identifier for Ed25519 (RFC 9052 section 3.1).
ALG_ED25519 = -19

#: Protected-header label for the algorithm identifier.
LABEL_ALG = 1

#: Protected-header label for the key identifier.
LABEL_KID = 4

#: Protected-header label for critical headers; this profile rejects it.
LABEL_CRIT = 2

#: Context string of the RFC 9052 `Sig_structure`.
SIGNATURE_CONTEXT = "Signature1"

#: External additional authenticated data bound into every signature.
EXTERNAL_AAD = b"wdbx-cose-sign1-p1"

#: Length of the key-identifier byte string.
KEY_ID_BYTES = 72

#: Length of an Ed25519 signature.
SIGNATURE_BYTES = 64

#: Length of the signed payload: the 32-byte episode digest.
PAYLOAD_BYTES = 32

#: Largest accepted encoded container, in bytes.
MAX_CONTAINER_BYTES = 512

#: Largest accepted encoded protected header, in bytes.
MAX_PROTECTED_BYTES = 128

#: Prefix of every derived signer key identifier.
SIGNER_KEY_ID_PREFIX = "ed25519:"

#: Number of lowercase hex digits after the prefix.
SIGNER_KEY_ID_HEX = 64

#: Nesting limit of the shared canonical reader (src/v3/commitment/decode.rs).
MAX_NESTING_DEPTH = 32

#: Largest signed integer of the canonical reader.
I64_MAX = (1 << 63) - 1

#: The scope label every response carries, so no reader can mistake this
#: witness for a signature verifier.
SCOPE = "encoding-agreement-only"

DEFAULT_GOLDEN_DIR = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    os.pardir,
    "crates",
    "abi-wdbx",
    "tests",
    "golden",
    "cose-sign1-p1",
)

#: Longest accepted request line, including its terminator.
MAX_LINE_BYTES = 65_536

#: Reader policy for the protected header, mirroring `protected_limits()`.
PROTECTED_LIMITS = {
    "max_input_bytes": MAX_PROTECTED_BYTES,
    "max_value_count": 16,
    "max_collection_entries": 8,
    "max_string_bytes": MAX_PROTECTED_BYTES,
    "max_parent_count": 0,
}


class CoseError(Exception):
    """A content-free container rejection.

    `name` mirrors one Rust `CoseError` variant exactly, so the two sides can
    be compared name for name. No bytes, key identifiers or digests are kept.
    """

    def __init__(self, name: str) -> None:
        super().__init__(name)
        self.name = name


class CanonicalError(Exception):
    """An internal canonical-reader failure.

    The Rust COSE reader collapses every failure of the shared canonical reader
    into `MalformedProtectedHeader`, so this class never escapes the protected
    header parser.
    """

    def __init__(self, name: str) -> None:
        super().__init__(name)
        self.name = name


# --- canonical encoding ------------------------------------------------------


def encode_head(major: int, argument: int) -> bytes:
    """Encode one CBOR head with the shortest argument form."""
    if argument < 0:
        raise ValueError("negative argument")
    if argument <= 23:
        return bytes(((major << 5) | argument,))
    if argument <= 0xFF:
        return bytes(((major << 5) | 0x18, argument))
    if argument <= 0xFFFF:
        return bytes(((major << 5) | 0x19,)) + argument.to_bytes(2, "big")
    if argument <= 0xFFFFFFFF:
        return bytes(((major << 5) | 0x1A,)) + argument.to_bytes(4, "big")
    return bytes(((major << 5) | 0x1B,)) + argument.to_bytes(8, "big")


def encode_bytes(value: bytes) -> bytes:
    """Encode a definite-length byte string."""
    return encode_head(2, len(value)) + value


def encode_text(value: str) -> bytes:
    """Encode a definite-length UTF-8 text string."""
    raw = value.encode("utf-8")
    return encode_head(3, len(raw)) + raw


def encode_integer(value: int) -> bytes:
    """Encode an integer; negative values use the CBOR negative major type."""
    if value >= 0:
        return encode_head(0, value)
    return encode_head(1, -1 - value)


def encode_map(entries: list) -> bytes:
    """Encode a canonical map: keys sorted by encoded length then bytes."""
    encoded = []
    for key, value in entries:
        encoded.append((encode_key(key), encode_key(value)))
    encoded.sort(key=lambda pair: (len(pair[0]), pair[0]))
    for left, right in zip(encoded, encoded[1:]):
        if left[0] == right[0]:
            raise ValueError("duplicate canonical map key")
    out = bytearray(encode_head(5, len(encoded)))
    for key, value in encoded:
        out += key
        out += value
    return bytes(out)


def encode_key(value) -> bytes:
    """Encode one canonical value; only the profile's own shapes are needed."""
    if isinstance(value, bool):
        return b"\xf5" if value else b"\xf4"
    if value is None:
        return b"\xf6"
    if isinstance(value, int):
        return encode_integer(value)
    if isinstance(value, (bytes, bytearray)):
        return encode_bytes(bytes(value))
    if isinstance(value, str):
        return encode_text(value)
    if isinstance(value, (list, tuple)):
        out = bytearray(encode_head(4, len(value)))
        for item in value:
            out += encode_key(item)
        return bytes(out)
    if isinstance(value, dict):
        return encode_map(list(value.items()))
    raise ValueError("unsupported canonical value")


# --- canonical reading -------------------------------------------------------


class CanonicalReader:
    """Bounded, non-normalizing reader for the shared canonical profile.

    This mirrors `src/v3/commitment/decode.rs` closely enough that the
    protected header this reader accepts is exactly the one Rust accepts. It
    is used only for the protected header, and every failure it raises is
    collapsed into `MalformedProtectedHeader` by the caller, exactly as Rust
    does.
    """

    def __init__(self, data: bytes, limits: dict) -> None:
        self.data = data
        self.cursor = 0
        self.remaining_values = limits["max_value_count"]
        self.limits = limits

    def decode(self):
        """Decode exactly one complete canonical value."""
        if len(self.data) > self.limits["max_input_bytes"]:
            raise CanonicalError("InputLimit")
        value = self.value(0)
        if self.cursor != len(self.data):
            raise CanonicalError("TrailingBytes")
        return value

    def _bytes(self, length: int) -> bytes:
        end = self.cursor + length
        if end > len(self.data):
            raise CanonicalError("UnexpectedEnd")
        chunk = self.data[self.cursor : end]
        self.cursor = end
        return chunk

    def _argument(self, info: int) -> int:
        if info <= 23:
            return info
        if info == 24:
            width, minimum = 1, 24
        elif info == 25:
            width, minimum = 2, 256
        elif info == 26:
            width, minimum = 4, 65_536
        elif info == 27:
            width, minimum = 8, 4_294_967_296
        else:
            raise CanonicalError("InvalidAdditionalInformation")
        argument = int.from_bytes(self._bytes(width), "big")
        if argument < minimum:
            raise CanonicalError("NonMinimalArgument")
        return argument

    def _collection(self, length: int, multiplier: int) -> None:
        if length > self.limits["max_collection_entries"]:
            raise CanonicalError("CollectionLimit")
        children = length * multiplier
        if children > self.remaining_values:
            raise CanonicalError("ValueLimit")
        # Every child needs at least one byte, so an impossible declaration is
        # refused before anything is allocated.
        if children > len(self.data) - self.cursor:
            raise CanonicalError("UnexpectedEnd")

    def value(self, depth: int):
        """Decode one canonical value as a `(kind, payload)` pair."""
        if depth > MAX_NESTING_DEPTH:
            raise CanonicalError("NestingLimit")
        if self.remaining_values <= 0:
            raise CanonicalError("ValueLimit")
        self.remaining_values -= 1
        initial = self._bytes(1)[0]
        major = initial >> 5
        info = initial & 31
        if major == 7:
            if initial == 0xF4:
                return ("bool", False)
            if initial == 0xF5:
                return ("bool", True)
            if initial == 0xF6:
                return ("null", None)
            raise CanonicalError("UnsupportedType")
        if major == 6:
            raise CanonicalError("UnsupportedType")
        argument = self._argument(info)
        if major == 0:
            return ("uint", argument)
        if major == 1:
            if argument > I64_MAX:
                raise CanonicalError("IntegerOutOfRange")
            return ("neg", -1 - argument)
        if major in (2, 3):
            if argument > self.limits["max_string_bytes"]:
                raise CanonicalError("StringLimit")
            chunk = self._bytes(argument)
            if major == 2:
                return ("bytes", chunk)
            try:
                return ("text", chunk.decode("utf-8"))
            except UnicodeDecodeError:
                raise CanonicalError("InvalidUtf8") from None
        if major == 4:
            self._collection(argument, 1)
            items = []
            for _ in range(argument):
                items.append(self.value(depth + 1))
            return ("array", items)
        if major == 5:
            self._collection(argument, 2)
            entries = []
            previous = None
            for _ in range(argument):
                start = self.cursor
                key = self.value(depth + 1)
                raw = self.data[start : self.cursor]
                if previous is not None:
                    if previous == raw:
                        raise CanonicalError("DuplicateMapKey")
                    if len(previous) > len(raw) or (
                        len(previous) == len(raw) and previous > raw
                    ):
                        raise CanonicalError("MapKeyOrder")
                previous = raw
                entries.append((key, self.value(depth + 1)))
            return ("map", entries)
        raise CanonicalError("UnsupportedType")


def decode_canonical(data: bytes, limits: dict):
    """Decode one complete canonical value or raise `CanonicalError`."""
    return CanonicalReader(data, limits).decode()


# --- the profile's own readers ------------------------------------------------


class ContainerReader:
    """Bounded reader over one `COSE_Sign1` container.

    Mirrors `src/v3/episode/cose/decode.rs`: definite lengths only, shortest
    argument encodings only, and no tolerance for anything but the profile's
    exact shape.
    """

    def __init__(self, data: bytes) -> None:
        self.data = data
        self.cursor = 0

    def head(self):
        """Read one CBOR head and return its major type and argument."""
        if self.cursor >= len(self.data):
            raise CoseError("Truncated")
        initial = self.data[self.cursor]
        self.cursor += 1
        major = initial >> 5
        info = initial & 31
        if info <= 23:
            argument = info
        elif info == 24:
            argument = self.short_argument(1, 24)
        elif info == 25:
            argument = self.short_argument(2, 256)
        elif info == 26:
            argument = self.short_argument(4, 65_536)
        elif info == 27:
            argument = self.short_argument(8, 4_294_967_296)
        else:
            raise CoseError("UnexpectedStructure")
        return major, argument

    def short_argument(self, width: int, minimum: int) -> int:
        """Read a fixed-width argument and reject a non-shortest encoding."""
        argument = int.from_bytes(self.take(width), "big")
        if argument < minimum:
            raise CoseError("NonMinimalArgument")
        return argument

    def take(self, length: int) -> bytes:
        """Borrow the next `length` bytes."""
        end = self.cursor + length
        if end > len(self.data):
            raise CoseError("Truncated")
        chunk = self.data[self.cursor : end]
        self.cursor = end
        return chunk

    def capped_byte_string(self, cap: int) -> bytes:
        """Read one definite-length byte string of at most `cap` bytes."""
        major, length = self.head()
        if major != 2:
            raise CoseError("UnexpectedStructure")
        if length > cap:
            raise CoseError("ProtectedHeaderTooLarge")
        return self.take(length)

    def exact_byte_string(self, expected: int, mismatch: str) -> bytes:
        """Read one definite-length byte string of exactly `expected` bytes."""
        major, length = self.head()
        if major != 2:
            raise CoseError("UnexpectedStructure")
        if length != expected:
            raise CoseError(mismatch)
        return self.take(length)


def is_signer_key_id(text: str) -> bool:
    """Whether `text` is exactly a derived `SignerKeyId`.

    Mirrors `SignerKeyId::parse`: the `ed25519:` prefix followed by exactly 64
    lowercase hex digits, and nothing else.
    """
    if not text.startswith(SIGNER_KEY_ID_PREFIX):
        return False
    digits = text[len(SIGNER_KEY_ID_PREFIX) :]
    if len(digits) != SIGNER_KEY_ID_HEX:
        return False
    return all(character in "0123456789abcdef" for character in digits)


def parse_protected_header(protected: bytes):
    """Parse the protected header and return `(alg, kid, key_id)`.

    Mirrors `decode.rs::protected_header`, including the order in which the
    labels are inspected, so both sides refuse the same container with the same
    error name.
    """
    try:
        header = decode_canonical(protected, PROTECTED_LIMITS)
    except CanonicalError:
        raise CoseError("MalformedProtectedHeader") from None
    if header[0] != "map":
        raise CoseError("MalformedProtectedHeader")

    algorithm = None
    key_id_bytes = None
    for key, value in header[1]:
        if key[0] != "uint":
            raise CoseError("MalformedProtectedHeader")
        label = key[1]
        if label == LABEL_ALG:
            if value[0] != "neg":
                raise CoseError("UnsupportedAlgorithm")
            algorithm = value[1]
        elif label == LABEL_KID:
            if value[0] != "bytes":
                raise CoseError("InvalidKeyId")
            key_id_bytes = value[1]
        elif label == LABEL_CRIT:
            raise CoseError("CriticalHeaderUnsupported")
        else:
            raise CoseError("UnknownProtectedLabel")

    if algorithm is None:
        raise CoseError("MalformedProtectedHeader")
    if algorithm != ALG_ED25519:
        raise CoseError("UnsupportedAlgorithm")
    if key_id_bytes is None:
        raise CoseError("MalformedProtectedHeader")
    if len(key_id_bytes) != KEY_ID_BYTES:
        raise CoseError("InvalidKeyId")
    try:
        text = key_id_bytes.decode("utf-8")
    except UnicodeDecodeError:
        raise CoseError("InvalidKeyId") from None
    if not is_signer_key_id(text):
        raise CoseError("InvalidKeyId")
    return algorithm, key_id_bytes, text


def decode_container(data: bytes) -> dict:
    """Decode one `COSE_Sign1` container.

    This checks syntax and shape only. It says nothing about trust,
    authorization, admission, or whether the signature verifies.
    """
    if len(data) > MAX_CONTAINER_BYTES:
        raise CoseError("ContainerTooLarge")
    reader = ContainerReader(data)

    major, tag = reader.head()
    if major != 6 or tag != COSE_SIGN1_TAG:
        raise CoseError("UnexpectedTag")
    major, count = reader.head()
    if major != 4:
        raise CoseError("UnexpectedStructure")
    if count != 4:
        raise CoseError("UnexpectedElementCount")

    protected = reader.capped_byte_string(MAX_PROTECTED_BYTES)
    major, count = reader.head()
    if major != 5:
        raise CoseError("UnexpectedStructure")
    if count != 0:
        raise CoseError("UnprotectedHeaderNotEmpty")

    payload = reader.exact_byte_string(PAYLOAD_BYTES, "InvalidPayloadLength")
    signature = reader.exact_byte_string(SIGNATURE_BYTES, "InvalidSignatureLength")

    if reader.cursor != len(data):
        raise CoseError("TrailingBytes")

    algorithm, key_id_bytes, key_id = parse_protected_header(protected)
    return {
        "protected": protected,
        "alg": algorithm,
        "kid": key_id_bytes,
        "key_id": key_id,
        "payload": payload,
        "signature": signature,
    }


def build_sig_structure(protected: bytes, payload: bytes) -> bytes:
    """Rebuild the RFC 9052 `Sig_structure` this profile signs.

    `Sig_structure = ["Signature1", protected, external_aad, payload]`, encoded
    by the same canonical encoder the Rust side uses. The protected bytes are
    the ones the container carries, never a re-encoded map, so a container that
    carries different protected bytes rebuilds a different structure.
    """
    return (
        encode_head(4, 4)
        + encode_text(SIGNATURE_CONTEXT)
        + encode_bytes(protected)
        + encode_bytes(EXTERNAL_AAD)
        + encode_bytes(payload)
    )


def build_protected_header(algorithm: int, key_id_bytes: bytes) -> bytes:
    """Rebuild the canonical protected header `{1: alg, 4: kid}`."""
    return encode_map(
        [
            (LABEL_ALG, algorithm),
            (LABEL_KID, bytes(key_id_bytes)),
        ]
    )


def inspect(data: bytes) -> dict:
    """Parse one container and report the encoding facts a peer can compare.

    Nothing here is a verdict about the signature: `signature_bytes_hex` is the
    bytes the container carried, not a statement that they verify.
    """
    parsed = decode_container(data)
    structure = build_sig_structure(parsed["protected"], parsed["payload"])
    return {
        "ok": True,
        "scope": SCOPE,
        "protected_hex": parsed["protected"].hex(),
        "sig_structure_hex": structure.hex(),
        "kid": parsed["key_id"],
        "alg": parsed["alg"],
        "payload_hex": parsed["payload"].hex(),
        "signature_bytes_hex": parsed["signature"].hex(),
    }


# --- request framing ---------------------------------------------------------


def _reject(pairs):
    """Reject duplicate JSON object keys."""
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate key")
        result[key] = value
    return result


def parse_request(line: bytes):
    """Return the container bytes of one strict request line.

    The request is exactly `{"container_hex": "<lowercase hex>"}`. Anything else
    is a `RequestError`; the request layer never sees container bytes.
    """
    try:
        text = line.decode("utf-8")
    except UnicodeDecodeError:
        return None
    try:
        value = json.loads(text, object_pairs_hook=_reject)
    except ValueError:
        return None
    if not isinstance(value, dict) or set(value) != {"container_hex"}:
        return None
    encoded = value["container_hex"]
    if not isinstance(encoded, str):
        return None
    if len(encoded) % 2 != 0:
        return None
    if any(character not in "0123456789abcdef" for character in encoded):
        return None
    return bytes.fromhex(encoded)


def respond(line: bytes) -> dict:
    """Answer one request line with one strict response object."""
    data = parse_request(line)
    if data is None:
        return {"ok": False, "scope": SCOPE, "error": "RequestError"}
    try:
        return inspect(data)
    except CoseError as error:
        return {"ok": False, "scope": SCOPE, "error": error.name}


def iter_lines(source):
    """Yield bounded logical lines; an over-long line yields `None`."""
    while True:
        chunk = source.readline(MAX_LINE_BYTES + 1)
        if not chunk:
            return
        if len(chunk) > MAX_LINE_BYTES:
            # Drain the rest of the line so the next request still lines up.
            while not chunk.endswith(b"\n"):
                more = source.readline(MAX_LINE_BYTES + 1)
                if not more:
                    break
                chunk = more
            yield None
            continue
        yield chunk


def run_json_lines(source, sink) -> int:
    """Serve the JSON-lines encoding protocol."""
    for line in iter_lines(source):
        if line is None:
            response = {"ok": False, "scope": SCOPE, "error": "IngressLimit"}
        else:
            stripped = line.strip()
            if not stripped:
                continue
            response = respond(stripped)
        sink.write((json.dumps(response, separators=(",", ":")) + "\n").encode("utf-8"))
        sink.flush()
    return 0


def inspect_hex(text: str) -> int:
    """Parse one container given as hex and print one JSON object."""
    stripped = text.strip()
    if len(stripped) % 2 != 0 or any(
        character not in "0123456789abcdef" for character in stripped
    ):
        print(json.dumps({"ok": False, "scope": SCOPE, "error": "RequestError"}))
        return 1
    response = respond(stripped.encode("ascii"))
    print(json.dumps(response, separators=(",", ":")))
    return 0 if response["ok"] else 1


# --- golden verification -----------------------------------------------------

_FAILURES = 0


def check(description: str, passed: bool) -> None:
    """Report one golden check."""
    global _FAILURES
    if passed:
        print(f"ok: {description}")
    else:
        _FAILURES += 1
        print(f"FAIL: {description}")


def _read_text(path: str) -> str:
    with open(path, encoding="ascii") as handle:
        return handle.read().strip()


def _read_hex(path: str) -> bytes:
    return bytes.fromhex(_read_text(path))


def verify_goldens(directory: str) -> int:
    """Rebuild the golden vectors and compare them with the pinned files."""
    global _FAILURES
    _FAILURES = 0
    container_path = os.path.join(directory, "container.hex")
    try:
        container = _read_hex(container_path)
    except (OSError, ValueError) as error:
        print(f"FAIL: cannot read {container_path}: {error}")
        return 1

    check(
        "container: sha256(golden bytes) == container.sha256",
        hashlib.sha256(container).hexdigest()
        == _read_text(os.path.join(directory, "container.sha256")),
    )

    try:
        parsed = decode_container(container)
    except CoseError as error:
        print(f"FAIL: the golden container does not parse: {error.name}")
        return 1

    check(
        "container: protected bytes == protected.hex",
        parsed["protected"] == _read_hex(os.path.join(directory, "protected.hex")),
    )
    check(
        "container: payload bytes == payload.hex",
        parsed["payload"] == _read_hex(os.path.join(directory, "payload.hex")),
    )
    check(
        "container: kid bytes == kid.hex",
        parsed["kid"] == _read_hex(os.path.join(directory, "kid.hex")),
    )
    check(
        "container: alg == alg.txt",
        parsed["alg"] == int(_read_text(os.path.join(directory, "alg.txt"))),
    )
    check(
        "protected: the canonical header of {1: alg, 4: kid} == the carried bytes",
        build_protected_header(parsed["alg"], parsed["kid"]) == parsed["protected"],
    )

    structure = build_sig_structure(parsed["protected"], parsed["payload"])
    check(
        "sig_structure: the rebuilt structure == sig_structure.hex",
        structure == _read_hex(os.path.join(directory, "sig_structure.hex")),
    )
    check(
        "sig_structure: sha256(rebuilt structure) == sig_structure.sha256",
        hashlib.sha256(structure).hexdigest()
        == _read_text(os.path.join(directory, "sig_structure.sha256")),
    )

    print(f"{_FAILURES} failure(s)")
    return 1 if _FAILURES else 0


def main(argv: list) -> int:
    """Dispatch the witness's commands."""
    if len(argv) < 2 or argv[1] in {"-h", "--help"}:
        print(__doc__)
        return 0 if len(argv) >= 2 else 1
    command = argv[1]
    if command == "parse-json-lines":
        return run_json_lines(sys.stdin.buffer, sys.stdout.buffer)
    if command == "inspect-hex":
        if len(argv) != 3:
            print("inspect-hex needs exactly one hex argument", file=sys.stderr)
            return 2
        return inspect_hex(argv[2])
    if command == "verify-goldens":
        return verify_goldens(argv[2] if len(argv) > 2 else DEFAULT_GOLDEN_DIR)
    print(f"unknown command: {command}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
