//! Resolution of the ambient durable-store base path from the environment.
//!
//! The one resolver every consumer (the abi CLI, the abi MCP server) calls,
//! so an operator's `ABI_WDBX_*` settings mean the same thing everywhere.
//! Ports Zig `durable_store.resolveConfig`. Precedence:
//!
//! 1. `ABI_WDBX_PERSIST` falsey (`0`/`false`/`no`/`off`) wins outright, even
//!    over an explicit `ABI_WDBX_PATH`: it "gates whether writes are durable
//!    at all" (`abi/tests/golden/wdbx-format.md`).
//! 2. `ABI_WDBX_PATH`, where the `:memory:` sentinel means no store.
//! 3. Off Windows, `XDG_DATA_HOME/abi/wdbx`.
//! 4. `<home>/.abi/wdbx`.
//! 5. No store when no home is available either.
//!
//! The two ambient roots, `XDG_DATA_HOME` and the home directory, are explicit
//! inputs rather than read here: a caller's test build withholds both so its
//! tests can never fall back to the operator's live store.

use std::path::PathBuf;

use abi_foundation::env;

/// The `ABI_WDBX_PATH` value that selects no durable store.
pub const MEMORY_SENTINEL: &str = ":memory:";

/// Where the ambient durable store lives, or why there is none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreLocation {
    /// Open the durable store at this base path.
    Durable(PathBuf),
    /// Use no durable store, for the given reason.
    Memory(MemoryReason),
}

/// Why [`StoreLocation::Memory`] was chosen. Callers that cannot serve without
/// a durable store use it to say which setting refused them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryReason {
    /// `ABI_WDBX_PERSIST` is falsey.
    PersistDisabled,
    /// `ABI_WDBX_PATH` is the `:memory:` sentinel.
    Sentinel,
    /// Nothing is configured and no home directory is available.
    NoHome,
}

fn is_falsey(value: &str) -> bool {
    matches!(value, "0" | "false" | "no" | "off")
}

/// Pure resolution over explicit inputs; see the module docs for precedence.
#[must_use]
pub fn resolve_store_location(
    persist: Option<&str>,
    path: Option<&str>,
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> StoreLocation {
    if persist.is_some_and(is_falsey) {
        return StoreLocation::Memory(MemoryReason::PersistDisabled);
    }
    if let Some(path) = path {
        return if path == MEMORY_SENTINEL {
            StoreLocation::Memory(MemoryReason::Sentinel)
        } else {
            StoreLocation::Durable(PathBuf::from(path))
        };
    }
    if !cfg!(windows)
        && let Some(xdg) = xdg_data_home
    {
        return StoreLocation::Durable(PathBuf::from(xdg).join("abi").join("wdbx"));
    }
    match home {
        Some(home) => StoreLocation::Durable(PathBuf::from(home).join(".abi").join("wdbx")),
        None => StoreLocation::Memory(MemoryReason::NoHome),
    }
}

/// [`resolve_store_location`] with `ABI_WDBX_PERSIST` and `ABI_WDBX_PATH` read
/// from the process environment (through the `abi_foundation::env` registry,
/// so test overrides apply and an empty value counts as unset) and the
/// caller-supplied ambient roots.
#[must_use]
pub fn resolve_store_location_from_env(
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> StoreLocation {
    resolve_store_location(
        env::get(env::WDBX_PERSIST).as_deref(),
        env::get(env::WDBX_PATH).as_deref(),
        xdg_data_home,
        home,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(persist, path, xdg_data_home, home) -> expected`.
    type Row<'a> = (
        Option<&'a str>,
        Option<&'a str>,
        Option<&'a str>,
        Option<&'a str>,
        StoreLocation,
    );

    fn durable(path: &str) -> StoreLocation {
        StoreLocation::Durable(PathBuf::from(path))
    }

    #[test]
    fn resolution_follows_the_documented_precedence() {
        use MemoryReason::{NoHome, PersistDisabled, Sentinel};
        use StoreLocation::Memory;
        let rows: Vec<Row<'_>> = vec![
            (
                Some("0"),
                Some("/x"),
                None,
                Some("/h"),
                Memory(PersistDisabled),
            ),
            (
                Some("off"),
                Some("/x"),
                Some("/xdg"),
                Some("/h"),
                Memory(PersistDisabled),
            ),
            (
                Some("no"),
                Some(":memory:"),
                None,
                Some("/h"),
                Memory(PersistDisabled),
            ),
            (
                Some("false"),
                None,
                None,
                Some("/h"),
                Memory(PersistDisabled),
            ),
            (Some("0"), None, None, None, Memory(PersistDisabled)),
            // Only the exact lowercase falsey set disables; anything else persists.
            (Some("1"), Some("/x"), None, Some("/h"), durable("/x")),
            (Some("OFF"), Some("/x"), None, Some("/h"), durable("/x")),
            (
                None,
                Some(":memory:"),
                Some("/xdg"),
                Some("/h"),
                Memory(Sentinel),
            ),
            (None, Some("/x"), Some("/xdg"), Some("/h"), durable("/x")),
            (None, None, None, Some("/h"), durable("/h/.abi/wdbx")),
            (None, None, None, None, Memory(NoHome)),
        ];
        for (persist, path, xdg, home, expected) in rows {
            assert_eq!(
                resolve_store_location(persist, path, xdg, home),
                expected,
                "persist={persist:?} path={path:?} xdg={xdg:?} home={home:?}"
            );
        }
    }

    #[test]
    fn xdg_data_home_wins_over_home_off_windows_only() {
        let got = resolve_store_location(None, None, Some("/xdg"), Some("/h"));
        if cfg!(windows) {
            assert_eq!(got, durable("/h/.abi/wdbx"));
        } else {
            assert_eq!(got, durable("/xdg/abi/wdbx"));
        }
    }

    #[test]
    fn env_wrapper_reads_the_registry_and_treats_empty_as_unset() {
        let _guard = env::lock_for_test();
        env::set_override(env::WDBX_PERSIST, "0");
        env::set_override(env::WDBX_PATH, "/x");
        let disabled = resolve_store_location_from_env(None, Some("/h"));
        env::set_override(env::WDBX_PERSIST, "");
        env::set_override(env::WDBX_PATH, "");
        // The ambient XDG root comes only from the caller, never the process env.
        env::set_override("XDG_DATA_HOME", "/env-xdg");
        let fallback = resolve_store_location_from_env(None, Some("/h"));
        env::clear_override(env::WDBX_PERSIST);
        env::clear_override(env::WDBX_PATH);
        env::clear_override("XDG_DATA_HOME");

        assert_eq!(
            disabled,
            StoreLocation::Memory(MemoryReason::PersistDisabled)
        );
        assert_eq!(fallback, durable("/h/.abi/wdbx"));
    }
}
