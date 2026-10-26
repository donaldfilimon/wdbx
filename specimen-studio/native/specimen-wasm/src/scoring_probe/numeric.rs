use super::project::*;
use serde_json::{Value, json};
use specimen_kernel::{scanner_v3::Rational, scoring_v3::*};
fn whole(n: i128) -> Rational {
    Rational::new(n, 1).unwrap()
}
pub const SEED: u64 = 18_446_744_073_709_551_557;
pub fn cases() -> Value {
    let base = Policy::default();
    let specifications = [
        ("default-threshold", whole(50), base, [0, 1]),
        ("negative-sign", whole(50), base, [u32::MAX, 0]),
        ("positive-sign", whole(49), base, [u32::MAX, 1]),
        ("brainstorm", whole(49), Policy::brainstorm(), [u32::MAX, 1]),
        (
            "positive-atp",
            whole(50),
            Policy {
                atp: Atp {
                    valence: whole(1),
                    intensity: whole(1),
                },
                ..base
            },
            [0, 0],
        ),
        (
            "negative-atp",
            whole(55),
            Policy {
                atp: Atp {
                    valence: Rational::new(-1, 2).unwrap(),
                    intensity: whole(1),
                },
                ..base
            },
            [0, 1],
        ),
        (
            "below-neighbor",
            Rational::new(49999999, 1000000).unwrap(),
            Policy {
                jitter: false,
                ..base
            },
            [0, 0],
        ),
        (
            "above-neighbor",
            Rational::new(50000001, 1000000).unwrap(),
            Policy {
                jitter: false,
                ..base
            },
            [0, 0],
        ),
        ("upper-endpoint", whole(100), base, [u32::MAX, 0]),
        ("lower-endpoint", whole(-100), base, [u32::MAX, 1]),
        (
            "displaced-endpoint",
            whole(100),
            Policy {
                atp: Atp {
                    valence: whole(-1),
                    intensity: whole(1),
                },
                ..base
            },
            [u32::MAX, 0],
        ),
        (
            "crystallized",
            whole(50),
            Policy {
                strength: 100,
                ..base
            },
            [u32::MAX, 0],
        ),
        (
            "subthreshold-strength100",
            whole(49),
            Policy {
                strength: 100,
                ..base
            },
            [u32::MAX, 1],
        ),
        (
            "zero-strength",
            whole(60),
            Policy {
                strength: 0,
                ..base
            },
            [0, 1],
        ),
        (
            "zero-cap",
            whole(60),
            Policy {
                cap: whole(0),
                ..base
            },
            [0, 1],
        ),
        (
            "disabled",
            whole(60),
            Policy {
                jitter: false,
                ..base
            },
            [0, 1],
        ),
        (
            "fractional-cap",
            Rational::new(149, 3).unwrap(),
            Policy {
                cap: Rational::new(3, 2).unwrap(),
                ..base
            },
            [2147483648, 1],
        ),
        (
            "clamped-jitter",
            Rational::new(199, 2).unwrap(),
            base,
            [u32::MAX, 1],
        ),
    ];
    json!(specifications.into_iter().map(|(id,scan,p,draws)|{let mut t=DrawTape::new(SEED,id,&draws,2).unwrap();let out=numeric_policy(scan,p,&mut t).unwrap();json!({"id":id,"input":{"scan":fraction(scan),"policy":policy(p),"seed":SEED.to_string(),"stream":id,"draws":draws,"limit":2},"before":0,"after":t.consumed(),"output":score(&out)})}).collect::<Vec<_>>())
}
pub fn elections() -> Value {
    json!([
 ("zero",vec![0.0,-0.0],vec![true,true],0),
 ("singleton",vec![0.0,3.0,9.0],vec![true,true,false],u32::MAX),
 ("first-endpoint",vec![1.0,3.0,4.0],vec![true,true,true],0),
 ("last-endpoint",vec![1.0,3.0,4.0],vec![true,true,true],u32::MAX),
 ("interior",vec![1.0,3.0,4.0],vec![true,true,true],2147483648),
 ("ineligible",vec![99.0,1.0,3.0],vec![false,true,true],0),
 ].into_iter().map(|(id,weights,eligible,d)|{let ds=[d];let mut t=DrawTape::new(SEED,id,&ds,1).unwrap();let out=elect(&weights,&eligible,&mut t).unwrap();json!({"id":id,"input":{"weight_bits":weights.iter().map(|w|format!("{:016x}",w.to_bits())).collect::<Vec<_>>(),"eligible":eligible,"seed":SEED.to_string(),"stream":id,"draws":ds,"limit":1},"before":0,"after":t.consumed(),"output":election(&out)})}).collect::<Vec<_>>())
}
pub fn seeded() -> Value {
    json!({"seed":SEED.to_string(),"count":6,"values":seeded_draws(SEED,6).unwrap()})
}
