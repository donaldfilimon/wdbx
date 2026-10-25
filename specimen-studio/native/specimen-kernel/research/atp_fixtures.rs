use serde_json::{Value, json};

pub(super) struct Fixture {
    pub(super) id: String,
    pub(super) state: [u32; 3],
    pub(super) stimulus: [u32; 2],
    pub(super) urgency: bool,
    pub(super) elapsed: u32,
    pub(super) previous: Option<String>,
}
impl Fixture {
    fn new(id: &str, state: [u32; 3], stimulus: [u32; 2], urgency: bool, elapsed: u32) -> Self {
        Self { id: id.into(), state, stimulus, urgency, elapsed, previous: None }
    }
    pub(super) fn input(&self) -> Value {
        json!({"state":self.state.map(hex), "stimulus":self.stimulus.map(hex),
            "urgency":self.urgency,"elapsed":hex(self.elapsed)})
    }
}
pub(super) fn hex(bits: u32) -> String { format!("{bits:08x}") }
pub(super) fn fixtures() -> Vec<Fixture> {
    let z = [0;3];
    let mut fs = vec![
        Fixture::new("positive-zero",z,[0;2],false,0),
        Fixture::new("negative-zero",[0x80000000;3],[0x80000000;2],false,0x80000000),
        Fixture::new("dt0-charge",z,[0x3f800000;2],false,0),
        Fixture::new("zero-intensity",[0x3f000000,0x3e800000,0],[0xbf800000,0],false,0),
        Fixture::new("urgency",z,[0,0],true,0),
        Fixture::new("roundorder",z,[0x3dcccccd,0x3f666666],false,0),
        Fixture::new("addition-order",[0x3dcccccd,0x3e99999a,0],[0x3dcccccd,0x3f666666],true,0),
        Fixture::new("decay-180",[0x3f800000,0x3f000000,0],[0;2],false,0x43340000),
        Fixture::new("cap-3600",[0xbf800000,0x3f000000,0],[0;2],false,0x45610000),
        Fixture::new("cap-maxfinite",[0xbf800000,0x3f000000,0],[0;2],false,0x7f7fffff),
        Fixture::new("release-below",[0x3f000000,0x3f599999,0],[0;2],false,0),
        Fixture::new("release-exact",[0x3f000000,0x3f59999a,0],[0;2],false,0),
        Fixture::new("release-above",[0x3f000000,0x3f59999b,0],[0;2],false,0),
        Fixture::new("clamp-positive",[0x3f800000,0x3f800000,0],[0x3f800000;2],true,0),
        Fixture::new("clamp-negative",[0xbf800000,0x3f800000,0],[0xbf800000,0x3f800000],true,0),
        Fixture::new("cooldown-positive",[0x3f000000,0x3f000000,0x41f00000],[0x3f800000;2],true,0x41200000),
        Fixture::new("cooldown-dt0",[0x3f000000,0x3f000000,0x3f800000],[0x3f800000;2],true,0),
        Fixture::new("cooldown-cross",[0,0,0x3f800000],[0x3f800000;2],true,0x40000000),
        Fixture::new("subnormal",[1,1,1],[1,1],true,1),
        Fixture::new("nearzero",[0x80800000,0x00800000,0],[0;2],false,0x45610000),
        Fixture::new("repeat-a",[0x3e800000,0x3e800000,0],[0x3dcccccd,0x3f666666],false,0x3f800000),
        Fixture::new("repeat-b",[0x3e800000,0x3e800000,0],[0x3dcccccd,0x3f666666],false,0x3f800000),
    ];
    let mut next = Fixture::new("cooldown-next",z,[0x3f800000;2],false,0);
    next.previous = Some("cooldown-cross".into()); fs.push(next);
    let mut previous = None;
    for (id, stimulus, urgency, elapsed) in [
        ("chain-idle-0",[0;2],false,0x3f800000),
        ("chain-idle-1",[0;2],false,0x3f800000),
        ("chain-build-0",[0x3f800000;2],true,0),
        ("chain-build-1",[0x3f800000;2],true,0),
        ("chain-release",[0x3f800000;2],true,0),
        ("chain-cooldown",[0x3f800000;2],true,0x41f00000),
        ("chain-resume",[0x3f800000;2],false,0),
    ] {
        let mut f=Fixture::new(id,z,stimulus,urgency,elapsed);
        f.previous=previous; previous=Some(id.into()); fs.push(f);
    }
    for (field, bad) in [
        ("state-v",[0xbf800001,0x3f800001]),
        ("state-a",[0x80000001,0x3f800001]),
        ("state-cooldown",[0x80000001,0x41f00001]),
        ("stimulus-v",[0xbf800001,0x3f800001]),
        ("stimulus-intensity",[0x80000001,0x3f800001]),
    ] {
        for (label,bits) in [ ("nan",0x7fc00000),("positive-inf",0x7f800000),
            ("negative-inf",0xff800000),("below",bad[0]),("above",bad[1]) ] {
            let mut f=Fixture::new(&format!("{field}-{label}"),z,[0;2],false,0);
            match field {
                "state-v" => f.state[0]=bits,
                "state-a" => f.state[1]=bits,
                "state-cooldown" => f.state[2]=bits,
                "stimulus-v" => f.stimulus[0]=bits,
                _ => f.stimulus[1]=bits,
            } fs.push(f);
        }
    }
    for (label,bits) in [("nan",0x7fc00000),("positive-inf",0x7f800000),
        ("negative-inf",0xff800000),("negative",0xbf800000),("negative-subnormal",0x80000001)] {
        fs.push(Fixture::new(&format!("elapsed-{label}"),z,[0;2],false,bits));
    }
    fs
}
