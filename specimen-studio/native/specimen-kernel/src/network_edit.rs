//! Editing commands for the sparse `neural::Network`: every command returns
//! a new network that passes `Network::validate`, or `InvalidNetwork`.
use crate::neural::{Layer, Network, Rng};
use crate::{Error, Result, error};
use serde::Deserialize;

/// Input features the first layer reads (`neural::encode`).
pub const INPUTS: usize = 128;
/// Outputs of the last layer (the composition decoder).
pub const DECODER_OUTPUTS: usize = 32;
pub const MAX_LAYERS: usize = 8;
pub const MAX_OUTPUTS: usize = 1024;
pub const MAX_WEIGHTS: usize = 131_072;

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "command",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NetworkCommand {
    /// Insert a hidden layer before layer `at` (never after the decoder).
    AddLayer {
        at: usize,
        outputs: usize,
        fan_in: usize,
        seed: u64,
    },
    /// Remove a hidden layer; the next layer is rewired to the new width.
    RemoveLayer { index: usize },
    /// Change a hidden layer's width; it and the next layer are rewired.
    ResizeLayer {
        index: usize,
        outputs: usize,
        seed: u64,
    },
    /// Rewire one layer with `fan_in` distinct inputs per neuron.
    SetConnectivity {
        index: usize,
        fan_in: usize,
        seed: u64,
    },
    /// Resample every weight, keeping connectivity; biases reset to 0.01.
    InitWeights { seed: u64 },
}

fn invalid(message: impl ToString) -> Error {
    error("InvalidNetwork", message)
}

/// Mean inputs per neuron of a layer (rounded).
pub fn fan_in(layer: &Layer) -> usize {
    if layer.outputs == 0 {
        0
    } else {
        (layer.weights.len() as f64 / layer.outputs as f64).round() as usize
    }
}

fn weight(rng: &mut Rng) -> f32 {
    ((rng.sample() - 0.5) * 0.7) as f32
}

/// A layer whose every neuron reads `fan_in` distinct inputs (capped at
/// `inputs`), chosen by a seeded partial Fisher-Yates shuffle and sorted.
fn sparse_layer(inputs: usize, outputs: usize, fan_in: usize, seed: u64) -> Result<Layer> {
    if outputs == 0 || outputs > MAX_OUTPUTS {
        return Err(invalid(format!("A layer needs 1 to {MAX_OUTPUTS} neurons")));
    }
    if fan_in == 0 {
        return Err(invalid("Each neuron needs at least one input"));
    }
    let fan_in = fan_in.min(inputs);
    if outputs * fan_in > MAX_WEIGHTS {
        return Err(invalid(format!(
            "{outputs} neurons × {fan_in} inputs exceeds {MAX_WEIGHTS} weights"
        )));
    }
    let mut rng = Rng(seed.max(1));
    let mut pool: Vec<usize> = (0..inputs).collect();
    let mut layer = Layer {
        inputs,
        outputs,
        offsets: vec![0],
        columns: Vec::with_capacity(outputs * fan_in),
        weights: Vec::with_capacity(outputs * fan_in),
        biases: vec![0.01; outputs],
    };
    for _ in 0..outputs {
        for i in 0..fan_in {
            let j = i + (rng.sample() * (inputs - i) as f64) as usize;
            pool.swap(i, j.min(inputs - 1));
        }
        let mut row = pool[..fan_in].to_vec();
        row.sort_unstable();
        for c in row {
            layer.columns.push(c);
            layer.weights.push(weight(&mut rng));
        }
        layer.offsets.push(layer.weights.len());
    }
    Ok(layer)
}

fn width_before(net: &Network, index: usize) -> usize {
    if index == 0 {
        INPUTS
    } else {
        net.layers[index - 1].outputs
    }
}

/// Rebuilds layer `index` for its (possibly new) input width, keeping its fan-in.
fn rewire(net: &mut Network, index: usize, seed: u64) -> Result<()> {
    if let Some(layer) = net.layers.get(index) {
        let inputs = width_before(net, index);
        net.layers[index] = sparse_layer(inputs, layer.outputs, fan_in(layer).max(1), seed)?;
    }
    Ok(())
}

fn hidden(net: &Network, index: usize) -> Result<()> {
    if index + 1 >= net.layers.len() {
        return Err(invalid(format!(
            "Layer {index} is the decoder or does not exist; it keeps {DECODER_OUTPUTS} outputs"
        )));
    }
    Ok(())
}

/// Applies `command` to a copy of `net`; the result always validates.
pub fn apply(net: &Network, command: NetworkCommand) -> Result<Network> {
    // Untrusted input: widths drive allocation, so validate before any work.
    net.validate().map_err(|e| invalid(e.message))?;
    let mut next = net.clone();
    match command {
        NetworkCommand::AddLayer {
            at,
            outputs,
            fan_in,
            seed,
        } => {
            if at >= next.layers.len() {
                return Err(invalid("A new layer goes before the decoder"));
            }
            if next.layers.len() >= MAX_LAYERS {
                return Err(invalid(format!(
                    "A network has at most {MAX_LAYERS} layers"
                )));
            }
            let layer = sparse_layer(width_before(&next, at), outputs, fan_in, seed)?;
            next.layers.insert(at, layer);
            rewire(&mut next, at + 1, seed.wrapping_add(1))?;
        }
        NetworkCommand::RemoveLayer { index } => {
            hidden(&next, index)?;
            next.layers.remove(index);
            rewire(&mut next, index, index as u64 + 1)?;
        }
        NetworkCommand::ResizeLayer {
            index,
            outputs,
            seed,
        } => {
            hidden(&next, index)?;
            let inputs = width_before(&next, index);
            let fan = fan_in(&next.layers[index]).max(1);
            next.layers[index] = sparse_layer(inputs, outputs, fan, seed)?;
            rewire(&mut next, index + 1, seed.wrapping_add(1))?;
        }
        NetworkCommand::SetConnectivity {
            index,
            fan_in,
            seed,
        } => {
            let layer = next
                .layers
                .get(index)
                .ok_or_else(|| invalid(format!("There is no layer {index}")))?;
            let outputs = layer.outputs;
            next.layers[index] = sparse_layer(width_before(&next, index), outputs, fan_in, seed)?;
        }
        NetworkCommand::InitWeights { seed } => {
            let mut rng = Rng(seed.max(1));
            for layer in &mut next.layers {
                for w in &mut layer.weights {
                    *w = weight(&mut rng);
                }
                layer.biases.iter_mut().for_each(|b| *b = 0.01);
            }
        }
    }
    next.validate().map_err(|e| invalid(e.message))?;
    Ok(next)
}
