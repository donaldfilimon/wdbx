#!/usr/bin/env python3
"""Second-language implementation of the `abbey-cbor-episode-v1` profile.

This is an independent, standard-library-only Python reimplementation of the
deterministic-CBOR commitment profile in
`crates/abi-wdbx/src/v3/commitment.rs`. It exists to give the Rust encoder a
cross-language witness: the same value must produce the same envelope bytes and
the same SHA-256 digest here and in Rust, and the same malformed inputs must be
refused with the same error names.

Scope, stated so nobody infers more from the file name:

- It reimplements the profile encoder, the five-entry envelope, and the digest.
- It does NOT derive an episode's header/payload from an `EpisodeWrite`; that
  mapping lives in the Rust store and is not reproduced here. So this verifies
  the canonical encoding, not a second-language episode store.
- It is not COSE, and it does not verify signatures.

Value model (the interchange form used by the Rust differential test and by
anyone else who wants to feed this script), one JSON object per value:

    {"u": 5}            unsigned integer, 0 ..= 2**64-1
    {"n": -3}           negative integer, -2**63 ..= -1 (zero or positive is refused)
    {"b": "0a0b"}       byte string, lowercase or uppercase hex
    {"t": "text"}       UTF-8 text string, encoded as its UTF-8 bytes, unnormalised
    {"a": [v, ...]}     definite-length array
    {"m": [[k, v], ...]} definite-length map, entries in the order supplied
    {"bool": true}      boolean
    {"null": true}      CBOR null

Commands:

    verify-goldens [DIR]   rebuild the two synthetic fixtures described in
                           DIR/README.md and compare with DIR/*.hex and *.sha256
                           (default DIR: crates/abi-wdbx/tests/golden/abbey-cbor-episode-v1)
    differential           read one JSON object per line from stdin:
                           {"schema_version": u, "header": V, "payload": V,
                            "parent_digests": ["<64 hex>", ...]}
                           and write one JSON object per line to stdout:
                           {"ok": true, "bytes": "<hex>", "digest": "<hex>"} or
                           {"ok": false, "error": "<RustErrorName>"}
    encode                 like `differential`, for a single object on stdin,
                           printing the envelope hex and digest on two lines

Exit status is 0 when every check passed and 1 otherwise. The script never
prints input values in error messages, mirroring the content-free Rust errors.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
from typing import Any

PROFILE_NAME = "abbey-cbor-episode-v1"
MAX_NESTING_DEPTH = 32
U64_MAX = (1 << 64) - 1
I64_MIN = -(1 << 63)

DEFAULT_GOLDEN_DIR = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    os.pardir,
    "crates",
    "abi-wdbx",
    "tests",
    "golden",
    "abbey-cbor-episode-v1",
)


class ProfileError(ValueError):
    """A profile violation. `name` mirrors the Rust `CanonicalCborError` variant."""

    def __init__(self, name: str) -> None:
        super().__init__(name)
        self.name = name


# --- value model -------------------------------------------------------------


class Unsigned:
    __slots__ = ("value",)

    def __init__(self, value: int) -> None:
        if not 0 <= value <= U64_MAX:
            raise ProfileError("LengthOutOfRange")
        self.value = value


class Negative:
    __slots__ = ("value",)

    def __init__(self, value: int) -> None:
        # Mirror Rust: the variant may hold any i64; zero or positive is refused
        # at encode time, not at construction time.
        if not I64_MIN <= value <= (1 << 63) - 1:
            raise ProfileError("LengthOutOfRange")
        self.value = value


class Bytes:
    __slots__ = ("value",)

    def __init__(self, value: bytes) -> None:
        self.value = bytes(value)


class Text:
    __slots__ = ("value",)

    def __init__(self, value: str) -> None:
        self.value = value


class Array:
    __slots__ = ("items",)

    def __init__(self, items: list[Any]) -> None:
        self.items = list(items)


class Map:
    __slots__ = ("entries",)

    def __init__(self, entries: list[tuple[Any, Any]]) -> None:
        self.entries = [(k, v) for k, v in entries]


class Bool:
    __slots__ = ("value",)

    def __init__(self, value: bool) -> None:
        self.value = bool(value)


class Null:
    __slots__ = ()


NULL = Null()


# --- encoder -------------------------------------------------------------------


def _argument(major: int, argument: int) -> bytes:
    """Shortest RFC 8949 encoding of an initial byte plus argument."""
    head = major << 5
    if argument <= 23:
        return bytes([head | argument])
    if argument <= 0xFF:
        return bytes([head | 0x18, argument])
    if argument <= 0xFFFF:
        return bytes([head | 0x19]) + argument.to_bytes(2, "big")
    if argument <= 0xFFFF_FFFF:
        return bytes([head | 0x1A]) + argument.to_bytes(4, "big")
    if argument <= U64_MAX:
        return bytes([head | 0x1B]) + argument.to_bytes(8, "big")
    raise ProfileError("LengthOutOfRange")


def encode_value(value: Any, depth: int = 0) -> bytes:
    if depth > MAX_NESTING_DEPTH:
        raise ProfileError("NestingLimit")
    if isinstance(value, Unsigned):
        return _argument(0, value.value)
    if isinstance(value, Negative):
        if value.value >= 0:
            raise ProfileError("NonNegativeValue")
        return _argument(1, -1 - value.value)
    if isinstance(value, Bytes):
        return _argument(2, len(value.value)) + value.value
    if isinstance(value, Text):
        raw = value.value.encode("utf-8")
        return _argument(3, len(raw)) + raw
    if isinstance(value, Array):
        out = bytearray(_argument(4, len(value.items)))
        for item in value.items:
            out += encode_value(item, depth + 1)
        return bytes(out)
    if isinstance(value, Map):
        return _encode_map(value.entries, depth)
    if isinstance(value, Bool):
        return b"\xf5" if value.value else b"\xf4"
    if isinstance(value, Null):
        return b"\xf6"
    raise TypeError("not a profile value")


def _encode_map(entries: list[tuple[Any, Any]], depth: int) -> bytes:
    encoded = []
    for key, value in entries:
        key_bytes = encode_value(key, depth + 1)
        value_bytes = encode_value(value, depth + 1)
        encoded.append((key_bytes, value_bytes))
    # RFC 8949 §4.2.1: shorter encoded key first, then bytewise.
    encoded.sort(key=lambda pair: (len(pair[0]), pair[0]))
    for left, right in zip(encoded, encoded[1:]):
        if left[0] == right[0]:
            raise ProfileError("DuplicateMapKey")
    out = bytearray(_argument(5, len(encoded)))
    for key_bytes, value_bytes in encoded:
        out += key_bytes
        out += value_bytes
    return bytes(out)


def envelope_bytes(
    schema_version: int, header: Any, payload: Any, parent_digests: list[bytes]
) -> bytes:
    """The domain-separated five-entry envelope, exactly as the Rust encoder builds it."""
    if schema_version == 0:
        raise ProfileError("ZeroSchemaVersion")
    if not isinstance(header, Map):
        raise ProfileError("HeaderMustBeMap")
    if not isinstance(payload, Map):
        raise ProfileError("PayloadMustBeMap")
    for digest in parent_digests:
        if len(digest) != 32:
            raise ProfileError("LengthOutOfRange")
    parents = Array([Bytes(d) for d in sorted(parent_digests)])
    envelope = Map(
        [
            (Unsigned(0), Text(PROFILE_NAME)),
            (Unsigned(1), Unsigned(schema_version)),
            (Unsigned(2), header),
            (Unsigned(3), payload),
            (Unsigned(4), parents),
        ]
    )
    return encode_value(envelope, 0)


def digest(envelope: bytes) -> bytes:
    return hashlib.sha256(envelope).digest()


# --- interchange form ----------------------------------------------------------


def value_from_json(obj: Any) -> Any:
    if not isinstance(obj, dict) or len(obj) != 1:
        raise ValueError("interchange value must be a one-key object")
    (tag, inner), = obj.items()
    if tag == "u":
        return Unsigned(int(inner))
    if tag == "n":
        return Negative(int(inner))
    if tag == "b":
        return Bytes(bytes.fromhex(inner))
    if tag == "t":
        return Text(str(inner))
    if tag == "a":
        return Array([value_from_json(item) for item in inner])
    if tag == "m":
        return Map([(value_from_json(k), value_from_json(v)) for k, v in inner])
    if tag == "bool":
        return Bool(bool(inner))
    if tag == "null":
        return NULL
    raise ValueError("unknown interchange tag")


def run_case(case: dict[str, Any]) -> dict[str, Any]:
    try:
        header = value_from_json(case["header"])
        payload = value_from_json(case["payload"])
        parents = [bytes.fromhex(p) for p in case.get("parent_digests", [])]
        env = envelope_bytes(int(case["schema_version"]), header, payload, parents)
    except ProfileError as error:
        return {"ok": False, "error": error.name}
    return {"ok": True, "bytes": env.hex(), "digest": digest(env).hex()}


# --- commands --------------------------------------------------------------------


def synthetic_fixture(parents: list[bytes]) -> bytes:
    """The fixture the golden README describes: schema 1, empty header, payload {1: "synthetic"}."""
    return envelope_bytes(1, Map([]), Map([(Unsigned(1), Text("synthetic"))]), parents)


def verify_goldens(directory: str) -> int:
    failures = 0

    def check(name: str, ok: bool) -> None:
        nonlocal failures
        print(f"{'ok  ' if ok else 'FAIL'} {name}")
        if not ok:
            failures += 1

    expected = {
        "empty-parents": synthetic_fixture([]),
        # Supplied in reverse order on purpose; the envelope must sort them.
        "two-parents": synthetic_fixture([b"\xee" * 32, b"\x11" * 32]),
    }
    for name, computed in expected.items():
        with open(os.path.join(directory, f"{name}.hex"), encoding="ascii") as handle:
            golden_hex = handle.read().strip()
        with open(os.path.join(directory, f"{name}.sha256"), encoding="ascii") as handle:
            golden_digest = handle.read().strip()
        golden_bytes = bytes.fromhex(golden_hex)
        check(f"{name}: python envelope bytes == golden .hex", computed == golden_bytes)
        check(f"{name}: sha256(python envelope) == golden .sha256", digest(computed).hex() == golden_digest)
        check(f"{name}: sha256(golden .hex bytes) == golden .sha256", digest(golden_bytes).hex() == golden_digest)

    # Boundary vectors pinned by the Rust unit tests, re-derived here.
    boundaries = [
        (Unsigned(23), "17"),
        (Unsigned(24), "1818"),
        (Unsigned(255), "18ff"),
        (Unsigned(256), "190100"),
        (Negative(-1), "20"),
        (Negative(-25), "3818"),
    ]
    for value, hex_bytes in boundaries:
        check(f"boundary {hex_bytes}", encode_value(value).hex() == hex_bytes)
    ordered = Map(
        [
            (Text("aa"), NULL),
            (Unsigned(24), NULL),
            (Text("b"), NULL),
            (Unsigned(1), NULL),
        ]
    )
    check(
        "map keys sort by encoded length then bytes",
        encode_value(ordered).hex() == "a401f61818f66162f6626161f6",
    )
    print(f"{failures} failure(s)")
    return 1 if failures else 0


def differential() -> int:
    status = 0
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            case = json.loads(line)
            result = run_case(case)
        except (ValueError, KeyError, TypeError):
            result = {"ok": False, "error": "MalformedInterchange"}
            status = 1
        sys.stdout.write(json.dumps(result, separators=(",", ":")) + "\n")
    sys.stdout.flush()
    return status


def encode_one() -> int:
    result = run_case(json.load(sys.stdin))
    if not result["ok"]:
        print(result["error"], file=sys.stderr)
        return 1
    print(result["bytes"])
    print(result["digest"])
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[1] in {"-h", "--help"}:
        print(__doc__)
        return 0 if len(argv) >= 2 else 1
    command = argv[1]
    if command == "verify-goldens":
        return verify_goldens(argv[2] if len(argv) > 2 else DEFAULT_GOLDEN_DIR)
    if command == "differential":
        return differential()
    if command == "encode":
        return encode_one()
    print(f"unknown command: {command}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
