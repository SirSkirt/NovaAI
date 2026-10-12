//! Neural Nova: self-contained dense neural network and training math.
//! No machine-learning crates; parameters and gradients are flat row-major f32 arrays.

/// Neural Nova v0.1.2: 64 recent ASCII characters, 16 fixed features each.
/// The 1024-dimensional network input and the 128-way ASCII output stay unchanged.
pub const CONTEXT_CHARS: usize = 64;
pub const FEATURES_PER_CHAR: usize = 16;
pub const ASCII_OUTPUTS: usize = 128;

/// Deterministic, from-scratch context encoder used by both GUI and CLI.
/// Left padding is encoded as all zeroes, distinct from a real space.
/// Feature channels: 7 bipolar ASCII bits, uppercase, lowercase, digit,
/// horizontal whitespace, punctuation, vowel, sentence boundary, newline, present.
pub fn encode_ascii_context(history: &[u8]) -> Result<Vec<f32>, String> {
    if history.iter().any(|byte| *byte >= ASCII_OUTPUTS as u8) {
        return Err("Neural Nova v0.1.2 accepts ASCII text only".to_string());
    }
    let mut features = vec![0.0; CONTEXT_CHARS * FEATURES_PER_CHAR];
    let recent = &history[history.len().saturating_sub(CONTEXT_CHARS)..];
    let first_position = CONTEXT_CHARS - recent.len();
    for (idx, &code) in recent.iter().enumerate() {
        let offset = (first_position + idx) * FEATURES_PER_CHAR;
        for bit in 0..7 {
            features[offset + bit] = if (code >> bit) & 1 == 1 { 1.0 } else { -1.0 };
        }
        features[offset + 7] = f32::from(code.is_ascii_uppercase());
        features[offset + 8] = f32::from(code.is_ascii_lowercase());
        features[offset + 9] = f32::from(code.is_ascii_digit());
        features[offset + 10] = f32::from(matches!(code, b' ' | b'\t' | b'\r'));
        features[offset + 11] = f32::from(code.is_ascii_punctuation());
        features[offset + 12] = f32::from(b"aeiouAEIOU".contains(&code));
        features[offset + 13] = f32::from(matches!(code, b'.' | b'!' | b'?' | b'\n'));
        features[offset + 14] = f32::from(code == b'\n');
        features[offset + 15] = 1.0;
    }
    Ok(features)
}

pub mod dialogue;

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const MAGIC: &[u8; 8] = b"NNOVA001";

#[derive(Clone, Debug)]
pub struct NetworkConfig {
    pub dimensions: Vec<usize>,
    pub seed: u64,
}

impl NetworkConfig {
    pub fn neural_nova_v1() -> Self {
        Self { dimensions: vec![1024, 512, 512, 128], seed: 0x4e4f_5641_2026 }
    }
}

#[derive(Clone, Debug)]
pub struct DenseLayer {
    pub inputs: usize,
    pub outputs: usize,
    /// Row-major: weights[output_neuron * inputs + input_neuron].
    pub weights: Vec<f32>,
    pub biases: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct Network {
    pub layers: Vec<DenseLayer>,
}

#[derive(Clone, Debug)]
pub struct LayerGrad {
    pub weights: Vec<f32>,
    pub biases: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct Gradients {
    pub layers: Vec<LayerGrad>,
}

#[derive(Clone, Debug)]
pub struct Adam {
    step: u32,
    learning_rate: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    m: Gradients,
    v: Gradients,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self { Self(if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed }) }
    fn next_f32(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let u = x.wrapping_mul(2685821657736338717);
        ((u >> 40) as f32) / (1u32 << 24) as f32
    }
}

impl Network {
    pub fn new(config: NetworkConfig) -> Result<Self, String> {
        let dims = &config.dimensions;
        if dims.len() < 2 || dims.len() > 17 ||
            dims.iter().any(|&n| n == 0 || n > 8192) {
            return Err("Need 2–17 layer dimensions, each between 1 and 8192".into());
        }
        let parameter_count: u64 = dims.windows(2)
            .map(|d| ((d[0] as u64) + 1) * (d[1] as u64)).sum();
        if parameter_count > 100_000_000 {
            return Err("Network exceeds the 100-million-parameter safety limit".into());
        }
        let mut rng = Rng::new(config.seed);
        let mut layers = Vec::with_capacity(dims.len() - 1);
        for pair in dims.windows(2) {
            let (inputs, outputs) = (pair[0], pair[1]);
            // He uniform initialization for ReLU hidden layers.
            let radius = (6.0f32 / inputs as f32).sqrt();
            let weights = (0..inputs * outputs)
                .map(|_| (rng.next_f32() * 2.0 - 1.0) * radius)
                .collect();
            layers.push(DenseLayer {
                inputs, outputs, weights, biases: vec![0.0; outputs],
            });
        }
        Ok(Self { layers })
    }

    pub fn dimensions(&self) -> Vec<usize> {
        let mut result = vec![self.layers[0].inputs];
        result.extend(self.layers.iter().map(|layer| layer.outputs));
        result
    }

    pub fn parameter_count(&self) -> usize {
        self.layers.iter().map(|l| l.weights.len() + l.biases.len()).sum()
    }

    /// Inspect real per-neuron activations. Includes the original input,
    /// all hidden layers, and output probabilities (4 columns for v0.1).
    pub fn activations(&self, input: &[f32]) -> Result<Vec<Vec<f32>>, String> {
        self.forward(input).map(|(activations, _)| activations)
    }

    /// Connection weight between a pair of neurons in adjacent layers.
    pub fn connection_weight(&self, transition: usize, from: usize, to: usize) -> Option<f32> {
        let layer = self.layers.get(transition)?;
        if from >= layer.inputs || to >= layer.outputs { return None; }
        layer.weights.get(to * layer.inputs + from).copied()
    }

    /// Every hidden layer uses ReLU; final output uses stable softmax.
    /// Returns probabilities whose sum is approximately 1.
    pub fn predict(&self, input: &[f32]) -> Result<Vec<f32>, String> {
        Ok(self.forward(input)?.0.pop().expect("forward has outputs"))
    }

    /// Returns a cross-entropy loss and gradients for a single example.
    pub fn loss_and_gradients(
        &self, input: &[f32], target: usize
    ) -> Result<(f32, Gradients), String> {
        let mut grads = Gradients::zeros(self);
        let loss = self.accumulate_example(input, target, &mut grads)?;
        Ok((loss, grads))
    }

    /// Averages gradients across examples, globally clips them, and takes one Adam step.
    pub fn train_batch(
        &mut self,
        examples: &[(&[f32], usize)],
        optimizer: &mut Adam,
    ) -> Result<f32, String> {
        if examples.is_empty() { return Err("Batch must not be empty".into()); }
        let mut gradients = Gradients::zeros(self);
        let mut loss = 0.0;
        for &(input, target) in examples {
            loss += self.accumulate_example(input, target, &mut gradients)?;
        }
        let inverse_batch = 1.0 / examples.len() as f32;
        gradients.scale(inverse_batch);
        gradients.clip_global_norm(1.0);
        optimizer.apply(self, &gradients)?;
        Ok(loss * inverse_batch)
    }

    fn forward(&self, input: &[f32]) -> Result<(Vec<Vec<f32>>, Vec<Vec<f32>>), String> {
        if input.len() != self.layers[0].inputs {
            return Err(format!("Expected {} features, got {}", self.layers[0].inputs, input.len()));
        }
        if input.iter().any(|v| !v.is_finite()) {
            return Err("Input contains NaN or infinity".into());
        }
        let mut activations = vec![input.to_vec()];
        let mut preactivations = Vec::with_capacity(self.layers.len());

        for (idx, layer) in self.layers.iter().enumerate() {
            let previous = activations.last().unwrap();
            let mut z = vec![0.0; layer.outputs];
            for (out, value) in z.iter_mut().enumerate() {
                let row = &layer.weights[out * layer.inputs..(out + 1) * layer.inputs];
                let mut sum = layer.biases[out];
                for (w, x) in row.iter().zip(previous) { sum += w * x; }
                *value = sum;
            }
            let final_layer = idx + 1 == self.layers.len();
            let a = if final_layer {
                softmax(&z)
            } else {
                z.iter().map(|&x| x.max(0.0)).collect()
            };
            preactivations.push(z);
            activations.push(a);
        }
        Ok((activations, preactivations))
    }

    fn accumulate_example(
        &self, input: &[f32], target: usize, gradients: &mut Gradients
    ) -> Result<f32, String> {
        if target >= self.layers.last().unwrap().outputs {
            return Err(format!("Target {target} out of output range"));
        }
        let (activations, preactivations) = self.forward(input)?;
        let probabilities = activations.last().unwrap();
        let loss = -probabilities[target].max(1.0e-30).ln();
        let mut delta = probabilities.clone();
        delta[target] -= 1.0; // d(softmax + cross entropy) / d(logits)

        for layer_idx in (0..self.layers.len()).rev() {
            let layer = &self.layers[layer_idx];
            let previous = &activations[layer_idx];
            let grad = &mut gradients.layers[layer_idx];

            for out in 0..layer.outputs {
                grad.biases[out] += delta[out];
                let row_start = out * layer.inputs;
                for i in 0..layer.inputs {
                    grad.weights[row_start + i] += delta[out] * previous[i];
                }
            }

            if layer_idx > 0 {
                let mut earlier = vec![0.0; layer.inputs];
                for out in 0..layer.outputs {
                    let row_start = out * layer.inputs;
                    for i in 0..layer.inputs {
                        earlier[i] += layer.weights[row_start + i] * delta[out];
                    }
                }
                for (i, item) in earlier.iter_mut().enumerate() {
                    if preactivations[layer_idx - 1][i] <= 0.0 { *item = 0.0; }
                }
                delta = earlier;
            }
        }
        Ok(loss)
    }

    /// Portable little-endian binary checkpoint. Includes network topology and parameters.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let mut file = File::create(path).map_err(|e| e.to_string())?;
        file.write_all(MAGIC).map_err(|e| e.to_string())?;
        let dims = self.dimensions();
        write_u32(&mut file, dims.len() as u32)?;
        for &d in &dims { write_u32(&mut file, d as u32)?; }
        for layer in &self.layers {
            for &x in layer.weights.iter().chain(layer.biases.iter()) {
                file.write_all(&x.to_le_bytes()).map_err(|e| e.to_string())?;
            }
        }
        file.flush().map_err(|e| e.to_string())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic != MAGIC { return Err("Not a Neural Nova checkpoint".into()); }
        let len = read_u32(&mut file)? as usize;
        if !(2..=17).contains(&len) { return Err("Invalid checkpoint layer count".into()); }
        let mut dims = Vec::with_capacity(len);
        for _ in 0..len { dims.push(read_u32(&mut file)? as usize); }
        let mut network = Self::new(NetworkConfig { dimensions: dims, seed: 1 })?;
        for layer in &mut network.layers {
            for x in layer.weights.iter_mut().chain(layer.biases.iter_mut()) {
                let mut bytes = [0u8; 4];
                file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                *x = f32::from_le_bytes(bytes);
                if !x.is_finite() { return Err("Checkpoint has invalid weights".into()); }
            }
        }
        let mut trailing = [0u8; 1];
        if file.read(&mut trailing).map_err(|e| e.to_string())? != 0 {
            return Err("Unexpected trailing checkpoint data".into());
        }
        Ok(network)
    }
}

fn softmax(values: &[f32]) -> Vec<f32> {
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut output: Vec<f32> = values.iter().map(|&x| (x - max).exp()).collect();
    let sum: f32 = output.iter().sum();
    for x in &mut output { *x /= sum; }
    output
}

fn write_u32(w: &mut impl Write, number: u32) -> Result<(), String> {
    w.write_all(&number.to_le_bytes()).map_err(|e| e.to_string())
}
fn read_u32(r: &mut impl Read) -> Result<u32, String> {
    let mut bytes = [0u8; 4];
    r.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Ok(u32::from_le_bytes(bytes))
}

impl Gradients {
    fn zeros(network: &Network) -> Self {
        Self {
            layers: network.layers.iter().map(|l| LayerGrad {
                weights: vec![0.0; l.weights.len()],
                biases: vec![0.0; l.biases.len()],
            }).collect()
        }
    }
    fn scale(&mut self, factor: f32) {
        for layer in &mut self.layers {
            for g in layer.weights.iter_mut().chain(layer.biases.iter_mut()) {
                *g *= factor;
            }
        }
    }
    fn clip_global_norm(&mut self, max_norm: f32) {
        let sq: f64 = self.layers.iter()
            .flat_map(|l| l.weights.iter().chain(l.biases.iter()))
            .map(|&g| (g as f64) * (g as f64)).sum();
        let norm = sq.sqrt();
        if norm > max_norm as f64 { self.scale((max_norm as f64 / norm) as f32); }
    }
}

impl Adam {
    pub fn new(network: &Network, learning_rate: f32) -> Result<Self, String> {
        if !learning_rate.is_finite() || learning_rate <= 0.0 {
            return Err("Learning rate must be a positive, finite number".into());
        }
        Ok(Self {
            step: 0, learning_rate, beta1: 0.9, beta2: 0.999, epsilon: 1e-8,
            m: Gradients::zeros(network), v: Gradients::zeros(network)
        })
    }

    fn apply(&mut self, network: &mut Network, gradient: &Gradients) -> Result<(), String> {
        if self.m.layers.len() != network.layers.len() ||
            self.m.layers.iter().zip(&network.layers).any(|(m, l)|
                m.weights.len() != l.weights.len() || m.biases.len() != l.biases.len()
            ) {
            return Err("Optimizer architecture does not match the network".into());
        }
        self.step = self.step.checked_add(1).ok_or("Adam step counter overflow")?;
        let c1 = 1.0 - self.beta1.powi(self.step as i32);
        let c2 = 1.0 - self.beta2.powi(self.step as i32);
        for (((layer, grad), m), v) in network.layers.iter_mut()
            .zip(&gradient.layers).zip(&mut self.m.layers).zip(&mut self.v.layers) {
            update_parameters(
                &mut layer.weights, &grad.weights, &mut m.weights, &mut v.weights,
                self.beta1, self.beta2, c1, c2, self.learning_rate, self.epsilon,
            );
            update_parameters(
                &mut layer.biases, &grad.biases, &mut m.biases, &mut v.biases,
                self.beta1, self.beta2, c1, c2, self.learning_rate, self.epsilon,
            );
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn update_parameters(
    params: &mut [f32], grads: &[f32], m: &mut [f32], v: &mut [f32],
    beta1: f32, beta2: f32, correction1: f32, correction2: f32,
    learning_rate: f32, epsilon: f32,
) {
    for i in 0..params.len() {
        let g = grads[i];
        m[i] = beta1 * m[i] + (1.0 - beta1) * g;
        v[i] = beta2 * v[i] + (1.0 - beta2) * g * g;
        let m_hat = m[i] / correction1;
        let v_hat = v[i] / correction2;
        params[i] -= learning_rate * m_hat / (v_hat.sqrt() + epsilon);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_architecture_has_correct_parameter_count() {
        let net = Network::new(NetworkConfig::neural_nova_v1()).unwrap();
        assert_eq!(net.dimensions(), vec![1024, 512, 512, 128]);
        assert_eq!(net.parameter_count(), 853_120);
        let p = net.predict(&vec![0.0; 1024]).unwrap();
        assert_eq!(p.len(), 128);
        assert!((p.iter().sum::<f32>() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn finite_difference_matches_backpropagation() {
        let net = Network::new(NetworkConfig {
            dimensions: vec![2, 4, 3], seed: 42,
        }).unwrap();
        let x = [0.3, -0.8];
        let (_loss, gradient) = net.loss_and_gradients(&x, 1).unwrap();
        let index = 1;
        let analytic = gradient.layers[1].weights[index];
        let epsilon = 1e-3;
        let mut plus = net.clone();
        plus.layers[1].weights[index] += epsilon;
        let mut minus = net.clone();
        minus.layers[1].weights[index] -= epsilon;
        let loss_plus = -plus.predict(&x).unwrap()[1].ln();
        let loss_minus = -minus.predict(&x).unwrap()[1].ln();
        let numeric = (loss_plus - loss_minus) / (2.0 * epsilon);
        assert!((analytic - numeric).abs() < 0.005,
            "analytic={analytic}, numeric={numeric}");
    }

    #[test]
    fn learns_simple_classification() {
        let mut net = Network::new(NetworkConfig {
            dimensions: vec![2, 16, 16, 2], seed: 23,
        }).unwrap();
        let mut adam = Adam::new(&net, 0.01).unwrap();
        let x0 = [1.0, 0.0];
        let x1 = [0.0, 1.0];
        let samples: [(&[f32], usize); 2] = [(&x0, 0), (&x1, 1)];
        let before = -net.predict(&x0).unwrap()[0].ln()
            - net.predict(&x1).unwrap()[1].ln();
        for _ in 0..250 { net.train_batch(&samples, &mut adam).unwrap(); }
        let after = -net.predict(&x0).unwrap()[0].ln()
            - net.predict(&x1).unwrap()[1].ln();
        assert!(after < before * 0.1, "loss: {before} -> {after}");
    }

    #[test]
    fn saves_and_restores_weights_exactly() {
        let net = Network::new(NetworkConfig {
            dimensions: vec![3, 5, 2], seed: 71,
        }).unwrap();
        let path = std::env::temp_dir().join(format!("neural-nova-{}.nnova", std::process::id()));
        net.save(&path).unwrap();
        let loaded = Network::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(net.dimensions(), loaded.dimensions());
        assert_eq!(net.predict(&[0.2, 0.5, -1.0]).unwrap(),
                   loaded.predict(&[0.2, 0.5, -1.0]).unwrap());
    }
    #[test]
    fn longer_context_preserves_older_characters_and_padding() {
        let short = encode_ascii_context(b"hello no").unwrap();
        let long = encode_ascii_context(b"there was once a very curious cat; hello no").unwrap();
        assert_eq!(short.len(), 1024);
        assert_eq!(short[..(CONTEXT_CHARS-8)*FEATURES_PER_CHAR], vec![0.0; 896]);
        assert_ne!(short, long);
        assert_eq!(&short[(CONTEXT_CHARS-8)*FEATURES_PER_CHAR..], &long[(CONTEXT_CHARS-8)*FEATURES_PER_CHAR..]);
        assert!(encode_ascii_context("café".as_bytes()).is_err());
    }

    #[test]
    fn context_encoding_distinguishes_real_spaces_from_padding() {
        let real_space = encode_ascii_context(b" ").unwrap();
        let empty = encode_ascii_context(b"").unwrap();
        assert_ne!(real_space, empty);
        assert_eq!(real_space[1023], 1.0);
        assert_eq!(empty[1023], 0.0);
    }

}