//! `wdbx`: native host commands over the WDBX v2 store.
//!
//! `wdbx typed inspect|commit` is the typed-host ledger contract a single
//! configured service writer uses for one private store directory (one store per
//! business object). The typed layer owns writer identity, contiguous typed
//! sequences under an exclusive lock, and HMAC admission envelopes keyed by a
//! private 32-byte host key; V2 journals underneath remain the durable,
//! hash-verified record. Errors go to stderr as `wdbx typed: <Name>: <detail>`
//! and never carry key bytes or stored values.

mod admission;
mod error;
mod files;
mod typed;

use error::TypedError;
use std::process::ExitCode;

const USAGE: &str = "usage: wdbx typed inspect <dir> <config> <host-key>\n       wdbx typed commit <dir> <config> <host-key> <auth.json> <transaction.json>";

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("wdbx typed: {}: {}", error.name, error.detail);
            ExitCode::from(error.code)
        }
    }
}

fn run() -> Result<String, TypedError> {
    let args = std::env::args_os()
        .skip(1)
        .map(|arg| arg.into_string().map_err(|_| TypedError::usage(USAGE)))
        .collect::<Result<Vec<_>, _>>()?;
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["typed", "inspect", dir, config, host_key] => {
            typed::inspect(&typed::Paths::new(dir, config, host_key)?)
        }
        ["typed", "commit", dir, config, host_key, auth, transaction] => typed::commit(
            &typed::Paths::new(dir, config, host_key)?,
            files::absolute(auth)?,
            files::absolute(transaction)?,
        ),
        _ => Err(TypedError::usage(USAGE)),
    }
}
