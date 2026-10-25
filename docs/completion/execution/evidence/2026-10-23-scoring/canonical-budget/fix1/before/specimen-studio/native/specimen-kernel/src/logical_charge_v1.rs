//! Canonical logical accounted bytes for the opt-in v3 budget mechanisms.
//!
//! These fixed v1 units are identical on native and WASM. They conservatively
//! reserve declared metadata categories, with nested collections and UTF-8
//! payload charged separately by each existing traversal. They are not measured
//! allocations, collection capacities, allocator overhead or a physical bound.
//! The reviewed arm64 reference fits every unit; Token, EntryRequest and
//! Contribution additionally round up. Type changes require reviewing this
//! explicit table, never deriving admission policy from the compiling layout.
use crate::{
    identity::Alternative,
    language::Expr,
    language_v3::{Capture, Charge, Definition, Span, Token},
    scanner_v3::routes::RouteCapture,
    scoring_v3::{Contribution, DrawRecord, EntryRequest, InhibitionFact},
};

pub(crate) trait LogicalChargeV1 {
    const UNIT_BYTES: usize;
}

pub(crate) const CONTRIBUTION_BYTES_V1: usize = 2048;

macro_rules! units {
    ($($ty:ty => $bytes:expr),+ $(,)?) => {
        $(impl LogicalChargeV1 for $ty {
            const UNIT_BYTES: usize = $bytes;
        })+
    };
}

// Concrete types only: no layout-derived or blanket generic implementation.
units! {
    u8 => 1,
    f64 => 8,
    usize => 8,
    u128 => 16,
    Span => 16,
    String => 24,
    Vec<Expr> => 24,
    Alternative => 40,
    Expr => 48,
    InhibitionFact => 32,
    DrawRecord => 48,
    Definition => 64,
    Charge => 56,
    Capture => 80,
    RouteCapture => 96,
    Token => 160,
    EntryRequest<'_> => 384,
    Contribution => CONTRIBUTION_BYTES_V1,
    &Contribution => 8,
}
