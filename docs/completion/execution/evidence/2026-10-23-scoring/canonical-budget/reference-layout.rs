use specimen_kernel::{identity::*, language::Expr, language_v3::*, scanner_v3::routes::*, scoring_v3::*};
fn main() {
 macro_rules! report { ($($t:ty),*) => { $(println!("{} {}", stringify!($t), std::mem::size_of::<$t>());)* }; }
 report!(u8, f64, usize, u128, Span, String, Vec<Expr>, Alternative, Expr, InhibitionFact, DrawRecord, Definition, Charge, Capture, RouteCapture, Token, EntryRequest<'static>, Contribution, &Contribution);
}
