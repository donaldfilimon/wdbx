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
- It also reimplements the store's derivation of an episode's header and
  payload from an `EpisodeWrite` in its JSON wire form (`episode_digest`
  below), so the pinned memory-candidate and memory-edge golden digests, and
  every event variant the Rust store accepts, can be reproduced without Rust.
- It does NOT reimplement the store: no transition rules, no replay or budget
  checks, no ledger. Given a write the store would refuse, it still returns
  the digest the store *would have computed*. The caller supplies the parent
  digest (the previous record of the same operation), which only the ledger
  knows.
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
    verify-episode-goldens [DIR]
                           derive the digests of DIR/episode_write_memory_candidate.json
                           and DIR/episode_write_memory_edge.json (no parent) and
                           compare with the digests the Rust tests pin
                           (default DIR: crates/abi-wdbx/tests/golden)
    episode-differential   read one JSON object per line from stdin:
                           {"write": <EpisodeWrite wire JSON>,
                            "previous_digest": "<64 hex>" | null}
                           and write {"ok": true, "digest": "<hex>"} or
                           {"ok": false, "error": "<name>"} per line
    episode-digest         like `episode-differential`, for a single object
                           on stdin, printing the digest on one line

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


# --- episode derivation (mirrors StoredRecord::computed_digest in store.rs) ----

# Digests the Rust golden tests pin (`GOLDEN_DIGEST` in
# tests/v3_memory_candidate.rs and tests/v3_memory_edge.rs). Both goldens open
# a fresh operation, so they have no parent.
EPISODE_GOLDENS = {
    "episode_write_memory_candidate.json": (
        "3c19a479a23077d95238b710876e5c03d2300dd77e9ccfedbbe6c11b0fc768bc"
    ),
    "episode_write_memory_edge.json": (
        "dfc839a6d6e6a39a0e3bccf837ea232f291ae74999241422efc26167eb47d47f"
    ),
}

DEFAULT_EPISODE_GOLDEN_DIR = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    os.pardir,
    "crates",
    "abi-wdbx",
    "tests",
    "golden",
)


class WireError(ValueError):
    """The wire JSON does not have the shape the Rust types serialise to."""


def _entry(key: str, value: Any) -> tuple[Any, Any]:
    return (Text(key), value)


def _digest32(value: Any) -> Bytes:
    if not (isinstance(value, list) and len(value) == 32 and all(isinstance(b, int) and 0 <= b <= 255 for b in value)):
        raise WireError("digest must be a 32-byte array")
    return Bytes(bytes(value))


def _opt_digest(value: Any) -> Any:
    return NULL if value is None else _digest32(value)


def _text(value: Any) -> Text:
    if not isinstance(value, str):
        raise WireError("expected a string")
    return Text(value)


def _unsigned(value: Any) -> Unsigned:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise WireError("expected an unsigned integer")
    return Unsigned(value)


def _bool(value: Any) -> Bool:
    if not isinstance(value, bool):
        raise WireError("expected a boolean")
    return Bool(value)


def _actor(actor: Any) -> Map:
    return Map([_entry("principal_id", _text(actor["principal_id"])), _entry("kind", _text(actor["kind"]))])


def _voice(voice: Any) -> Any:
    if voice is None:
        return NULL
    return Map(
        [
            _entry("consent_epoch", _unsigned(voice["consent_epoch"])),
            _entry("participant_count", _unsigned(voice["participant_count"])),
            _entry("authorization_state", _text(voice["authorization_state"])),
            _entry("attribution", _text(voice["attribution"])),
            _entry("stt", _text(voice["stt"])),
            _entry("tts", _text(voice["tts"])),
            _entry("playback", _text(voice["playback"])),
            _entry("barge_in_count", _unsigned(voice["barge_in_count"])),
            _entry("transitions", Array([_text(t) for t in voice["transitions"]])),
            _entry("terminal_reason", _text(voice["terminal_reason"])),
        ]
    )


def _candidate(candidate: Any) -> Map:
    dimension = candidate["dimension"]
    embedding_version = candidate["embedding_version"]
    return Map(
        [
            _entry("class", _text(candidate["class"])),
            _entry("retention", _text(candidate["retention"])),
            _entry("payload_commitment", _digest32(candidate["payload_commitment"])),
            _entry("payload_bytes", _unsigned(candidate["payload_bytes"])),
            _entry("dimension", NULL if dimension is None else _unsigned(dimension)),
            _entry("embedding_version", NULL if embedding_version is None else _text(embedding_version)),
            _entry("member_scoped", _bool(candidate["member_scoped"])),
            _entry("supersedes", _opt_digest(candidate["supersedes"])),
            _entry("forgets", _opt_digest(candidate["forgets"])),
        ]
    )


def _edge(edge: Any) -> Map:
    return Map(
        [
            _entry("kind", _text(edge["kind"])),
            _entry("target", _digest32(edge["target"])),
            _entry("counterpart", _opt_digest(edge["counterpart"])),
            _entry("reason", _text(edge["reason"])),
        ]
    )


def canonical_event(event: Any) -> Map:
    """`canonical_event` in store.rs, keyed by the serde `kind` tag."""
    kind = event["kind"]
    if kind == "proposal":
        return Map([_entry("requested_by", _actor(event["requested_by"])), _entry("proposed_by", _actor(event["proposed_by"]))])
    if kind == "approval":
        return Map([_entry("approved_by", _actor(event["approved_by"]))])
    if kind == "execution":
        return Map([_entry("executed_by", _actor(event["executed_by"])), _entry("voice", _voice(event["voice"]))])
    if kind == "compensation":
        return Map(
            [
                _entry("compensated_by", _actor(event["compensated_by"])),
                _entry("exact_restore_observed", _bool(event["exact_restore_observed"])),
            ]
        )
    if kind == "terminal":
        return Map([_entry("status", _text(event["status"])), _entry("reason", _text(event["reason"]))])
    if kind == "memory_candidate":
        return Map([_entry("recorded_by", _actor(event["recorded_by"])), _entry("candidate", _candidate(event["candidate"]))])
    if kind == "memory_edge":
        return Map([_entry("recorded_by", _actor(event["recorded_by"])), _entry("edge", _edge(event["edge"]))])
    raise WireError("unknown event kind")


def episode_digest(write: Any, previous_digest: bytes | None) -> bytes:
    """The digest `EpisodeStore` would commit for `write` after `previous_digest`.

    Mirrors `StoredRecord::computed_digest`: a schema-1 envelope whose header
    carries the nine binding fields, whose payload carries the event kind, the
    canonical event and the token cost, and whose parents are the previous
    record of the same operation (none when the write opens an operation).
    """
    consent_epoch = write["consent_epoch"]
    header = Map(
        [
            _entry("request_id", _text(write["request_id"])),
            _entry("operation_id", _text(write["operation_id"])),
            _entry("contract_revision", _unsigned(write["contract_revision"])),
            _entry("contract_digest", _digest32(write["contract_digest"])),
            _entry("guild_ref", _text(write["guild_ref"])),
            _entry("consent_epoch", NULL if consent_epoch is None else _unsigned(consent_epoch)),
            _entry("source_type", _text(write["source_type"])),
            _entry("policy_version", _text(write["policy_version"])),
            _entry("evidence_level", _text(write["evidence_level"])),
        ]
    )
    payload = Map(
        [
            _entry("event_kind", _text(write["event"]["kind"])),
            _entry("event", canonical_event(write["event"])),
            _entry("token_cost", _unsigned(write["token_cost"])),
        ]
    )
    parents = [] if previous_digest is None else [previous_digest]
    return digest(envelope_bytes(1, header, payload, parents))


def run_episode_case(case: dict[str, Any]) -> dict[str, Any]:
    try:
        previous = case.get("previous_digest")
        parent = None if previous is None else bytes.fromhex(previous)
        if parent is not None and len(parent) != 32:
            raise WireError("previous_digest must be 32 bytes")
        result = episode_digest(case["write"], parent)
    except ProfileError as error:
        return {"ok": False, "error": error.name}
    except (WireError, KeyError, TypeError, ValueError):
        return {"ok": False, "error": "MalformedWire"}
    return {"ok": True, "digest": result.hex()}


def verify_episode_goldens(directory: str) -> int:
    failures = 0
    for name, expected in EPISODE_GOLDENS.items():
        with open(os.path.join(directory, name), encoding="utf-8") as handle:
            write = json.load(handle)
        computed = episode_digest(write, None).hex()
        ok = computed == expected
        print(f"{'ok  ' if ok else 'FAIL'} {name}: python episode digest == pinned {expected[:8]}…")
        if not ok:
            failures += 1
    print(f"{failures} failure(s)")
    return 1 if failures else 0


def episode_differential() -> int:
    status = 0
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            result = run_episode_case(json.loads(line))
        except ValueError:
            result = {"ok": False, "error": "MalformedInterchange"}
            status = 1
        sys.stdout.write(json.dumps(result, separators=(",", ":")) + "\n")
    sys.stdout.flush()
    return status


def episode_one() -> int:
    result = run_episode_case(json.load(sys.stdin))
    if not result["ok"]:
        print(result["error"], file=sys.stderr)
        return 1
    print(result["digest"])
    return 0


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
    if command == "verify-episode-goldens":
        return verify_episode_goldens(argv[2] if len(argv) > 2 else DEFAULT_EPISODE_GOLDEN_DIR)
    if command == "episode-differential":
        return episode_differential()
    if command == "episode-digest":
        return episode_one()
    print(f"unknown command: {command}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
