//! Full stream/order records; no unbounded replay to authenticate history.
use super::{Cursor, Result};
use crate::model::{
    contract::training::{
        BatchPolicy, PendingState, RandomStream, RngOrderState, RngProfile, SamplerProfile,
        SeedProfile, StepBoundaryState, StreamId,
    },
    error::ErrorKind as E,
};
pub(super) fn parse(r: &mut Cursor<'_, '_>) -> Result<RngOrderState> {
    match r.u8()? {
        0 => {
            r.end()?;
            return Ok(RngOrderState::Absent);
        }
        1 => {}
        _ => return Err(r.error(E::NonCanonical, "state-tag")),
    }
    let profiles = [r.u16()?, r.u16()?];
    r.check(profiles == [1, 1], E::InvalidRngOrder, "rng-profiles")?;
    let root_seed = r.u64()?;
    let streams = read_streams(r, root_seed)?;
    let boundary_profiles = [r.u16()?, r.u16()?, r.u16()?];
    r.check(
        boundary_profiles == [1, 1, 0],
        E::InvalidRngOrder,
        "sampler-boundary",
    )?;
    let dataset_manifest_digest = r.digest()?;
    let split_manifest_digest = r.digest()?;
    let preparation_digest = r.digest()?;
    let successful_step = r.u64()?;
    let epoch = r.u64()?;
    let example_count = r.u32()?;
    let next_example_cursor = r.u32()?;
    let batch_examples = r.u32()?;
    let n = r.count(65536)?;
    let mut permutation = r.vec(n)?;
    for _ in 0..n {
        permutation.push(r.u32()?);
    }
    let accepted_topology_proposals = r.u64()?;
    let next_topology_proposal_step = r.u64()?;
    r.end()?;
    r.check(
        (1..=65536).contains(&example_count)
            && n == example_count as usize
            && next_example_cursor < example_count
            && (1..=4).contains(&batch_examples),
        E::InvalidRngOrder,
        "order-bounds",
    )?;
    let mut seen = r.vec::<u8>(n)?;
    seen.resize(n, 0);
    for i in &permutation {
        r.check(
            (*i as usize) < n && seen[*i as usize] == 0,
            E::InvalidRngOrder,
            "permutation",
        )?;
        seen[*i as usize] = 1;
    }
    r.check(
        successful_step < u64::MAX
            && (next_topology_proposal_step == 0 || next_topology_proposal_step > successful_step),
        E::InvalidRngOrder,
        "rng-counters",
    )?;
    // Box is fixed-size; all input-sized reservations above are fallible and budgeted.
    Ok(RngOrderState::StepBoundary(Box::new(StepBoundaryState {
        rng_profile: RngProfile::Xoshiro256StarStarV1,
        seed_profile: SeedProfile::SplitMix64V1,
        root_seed,
        streams,
        sampler_profile: SamplerProfile::FisherYatesUnbiasedU64V1,
        batch_policy: BatchPolicy::OrderedContiguous,
        pending_state: PendingState::None,
        dataset_manifest_digest,
        split_manifest_digest,
        preparation_digest,
        successful_step,
        epoch,
        example_count,
        next_example_cursor,
        batch_examples,
        permutation,
        accepted_topology_proposals,
        next_topology_proposal_step,
    })))
}
fn seed_stream(seed: u64, id: u8) -> [u64; 4] {
    let mut state = seed.wrapping_add(0x9e37_79b9_7f4a_7c15u64.wrapping_mul(u64::from(id)));
    std::array::from_fn(|_| {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    })
}

fn read_streams(r: &mut Cursor<'_, '_>, root_seed: u64) -> Result<Vec<RandomStream>> {
    let n = r.count(5)?;
    r.check(n == 5, E::InvalidRngOrder, "streams")?;
    let mut streams = r.vec(5)?;
    for expected in 1..=5 {
        let ident = r.u8()?;
        let state_words = [r.u64()?, r.u64()?, r.u64()?, r.u64()?];
        let consumed_u64 = r.u64()?;
        r.check(
            ident == expected && state_words != [0; 4] && consumed_u64 < u64::MAX,
            E::InvalidRngOrder,
            "stream-state",
        )?;
        r.check(
            consumed_u64 != 0 || state_words == seed_stream(root_seed, expected),
            E::InvalidRngOrder,
            "stream-initial-state",
        )?;
        streams.push(RandomStream {
            stream_id: StreamId::try_from(ident)?,
            state_words,
            consumed_u64,
        });
    }
    Ok(streams)
}
