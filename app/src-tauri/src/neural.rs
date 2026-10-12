use neural_nova::{Adam, Network, NetworkConfig};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering as CompareOrdering;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const WINDOW: usize = 8;
const ALPHABET: usize = 128;
const BATCH: usize = 8;
const MAX_TRAIN_BYTES: usize = 100_000;
const CHECKPOINT: &str = "neural-nova-v1.nnov";

pub struct BrainSession {
    network: Network,
    optimizer: Adam,
    steps: u64,
    examples: u64,
    last_loss: Option<f32>,
    trained: bool,
}

struct Shared {
    session: Mutex<BrainSession>,
    training: AtomicBool,
    cancel: AtomicBool,
}

#[derive(Clone)]
pub struct NeuralState {
    shared: Arc<Shared>,
}

impl Default for NeuralState {
    fn default() -> Self {
        let network = Network::new(NetworkConfig::neural_nova_v1()).expect("fixed neural topology");
        let optimizer = Adam::new(&network, 0.001).expect("valid learning rate");
        Self {
            shared: Arc::new(Shared {
                session: Mutex::new(BrainSession {
                    network, optimizer, steps: 0, examples: 0, last_loss: None,
                    trained: false,
                }),
                training: AtomicBool::new(false),
                cancel: AtomicBool::new(false),
            }),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrainStatus {
    dimensions: Vec<usize>,
    parameters: usize,
    steps: u64,
    examples: u64,
    last_loss: Option<f32>,
    trained: bool,
    training: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    transition: usize,
    from: usize,
    to: usize,
    weight: f32,
    contribution: f32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopPrediction {
    code: usize,
    probability: f32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BrainTrace {
    context: String,
    /// Exact input features, hidden activations, and output probabilities.
    layers: Vec<Vec<f32>>,
    /// Top connections only; values are actual current weights and contributions.
    links: Vec<Link>,
    top_predictions: Vec<TopPrediction>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TrainingProgress {
    epoch: u32,
    epochs: u32,
    completed_examples: usize,
    total_examples: usize,
    loss: f32,
    steps: u64,
    trace: BrainTrace,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainingRequest {
    text: String,
    epochs: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedText {
    text: String,
    trace: BrainTrace,
    status: BrainStatus,
}

fn lock_session(state: &NeuralState) -> Result<std::sync::MutexGuard<'_, BrainSession>, String> {
    state.shared.session.lock().map_err(|_| "Neural network lock poisoned".into())
}

fn checkpoint_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(CHECKPOINT))
}

fn status(state: &NeuralState, session: &BrainSession) -> BrainStatus {
    BrainStatus {
        dimensions: session.network.dimensions(),
        parameters: session.network.parameter_count(),
        steps: session.steps,
        examples: session.examples,
        last_loss: session.last_loss,
        trained: session.trained,
        training: state.shared.training.load(Ordering::Acquire),
    }
}

fn encode(context: &[u8; WINDOW]) -> Vec<f32> {
    let mut v = vec![0.0; WINDOW * ALPHABET];
    for (position, &byte) in context.iter().enumerate() {
        v[position * ALPHABET + byte as usize] = 1.0;
    }
    v
}

fn context_bytes(input: &str) -> Result<[u8; WINDOW], String> {
    if !input.is_ascii() {
        return Err("This first character model supports ASCII only. Unicode will come later.".into());
    }
    let mut value = [b' '; WINDOW];
    let bytes = input.as_bytes();
    let count = bytes.len().min(WINDOW);
    value[WINDOW - count..].copy_from_slice(&bytes[bytes.len() - count..]);
    Ok(value)
}

fn best_indices(values: &[f32], count: usize) -> Vec<usize> {
    let mut ids: Vec<usize> = (0..values.len()).filter(|&idx| values[idx] > 0.0).collect();
    ids.sort_unstable_by(|&a, &b| {
        values[b].partial_cmp(&values[a]).unwrap_or(CompareOrdering::Equal)
    });
    ids.truncate(count);
    ids
}

fn trace(network: &Network, context: &[u8; WINDOW]) -> Result<BrainTrace, String> {
    let layers = network.activations(&encode(context))?;
    let mut links = Vec::new();
    for transition in 0..network.layers.len() {
        let sources = best_indices(&layers[transition], 12);
        let destinations = best_indices(&layers[transition + 1], 9);
        for to in destinations {
            let mut candidates: Vec<Link> = sources.iter().filter_map(|&from| {
                let weight = network.connection_weight(transition, from, to)?;
                let contribution = weight * layers[transition][from];
                Some(Link { transition, from, to, weight, contribution })
            }).collect();
            candidates.sort_unstable_by(|a, b|
                b.contribution.abs().partial_cmp(&a.contribution.abs())
                    .unwrap_or(CompareOrdering::Equal)
            );
            links.extend(candidates.into_iter().take(2));
        }
    }
    let top_predictions = best_indices(&layers[3], 8).into_iter().map(|code| TopPrediction {
        code, probability: layers[3][code],
    }).collect();
    Ok(BrainTrace {
        context: String::from_utf8_lossy(context).to_string(),
        layers, links, top_predictions,
    })
}

#[tauri::command]
pub fn neural_boot(app: AppHandle, state: State<'_, NeuralState>) -> Result<BrainStatus, String> {
    if state.shared.training.load(Ordering::Acquire) {
        return Err("Cannot load while training is running".into());
    }
    let path = checkpoint_path(&app)?;
    let mut session = lock_session(&state)?;
    if path.exists() {
        let network = Network::load(&path)?;
        if network.dimensions() != vec![1024, 512, 512, 128] {
            return Err("Checkpoint belongs to a different neural architecture".into());
        }
        session.optimizer = Adam::new(&network, 0.001)?;
        session.network = network;
        session.trained = true;
        // Adam moments restart after loading a weights-only checkpoint.
    }
    Ok(status(&state, &session))
}

#[tauri::command]
pub fn neural_status(state: State<'_, NeuralState>) -> Result<BrainStatus, String> {
    let session = lock_session(&state)?;
    Ok(status(&state, &session))
}

#[tauri::command]
pub fn neural_inspect(state: State<'_, NeuralState>, text: String) -> Result<BrainTrace, String> {
    let context = context_bytes(&text)?;
    let session = lock_session(&state)?;
    trace(&session.network, &context)
}

fn sample_code(probabilities: &[f32], temperature: f32, random: f32) -> u8 {
    let mut weights = [0.0_f32; ALPHABET];
    let mut sum = 0.0;
    for code in 0..ALPHABET {
        if !(code == 10 || (32..=126).contains(&code)) { continue; }
        let p = probabilities[code].max(0.0).powf(1.0 / temperature);
        weights[code] = p;
        sum += p;
    }
    let mut needle = random * sum;
    for (i, &p) in weights.iter().enumerate() {
        needle -= p;
        if needle <= 0.0 && p > 0.0 { return i as u8; }
    }
    b' '
}

#[tauri::command]
pub async fn neural_generate(
    state: State<'_, NeuralState>,
    prompt: String,
    length: usize,
    temperature: f32,
) -> Result<GeneratedText, String> {
    if prompt.len() > 4096 || !prompt.is_ascii() {
        return Err("Prompt must be ASCII and at most 4,096 bytes".into());
    }
    if !(1..=240).contains(&length) {
        return Err("Generation length must be between 1 and 240".into());
    }
    if !temperature.is_finite() || !(0.2..=2.0).contains(&temperature) {
        return Err("Temperature must be between 0.2 and 2.0".into());
    }
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session = lock_session(&state)?;
        let mut all = prompt.into_bytes();
        let mut seed = SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64).unwrap_or(53);
        for _ in 0..length {
            let context = context_bytes(std::str::from_utf8(&all).map_err(|e| e.to_string())?)?;
            let probabilities = session.network.predict(&encode(&context))?;
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let random = (seed >> 40) as f32 / (1u32 << 24) as f32;
            all.push(sample_code(&probabilities, temperature, random));
        }
        let last_context = context_bytes(std::str::from_utf8(&all).map_err(|e| e.to_string())?)?;
        Ok(GeneratedText {
            text: String::from_utf8(all).map_err(|e| e.to_string())?,
            trace: trace(&session.network, &last_context)?,
            status: status(&state, &session),
        })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn neural_train(
    app: AppHandle,
    state: State<'_, NeuralState>,
    request: TrainingRequest,
) -> Result<(), String> {
    if !request.text.is_ascii() {
        return Err("Training currently supports ASCII text only".into());
    }
    if request.text.len() <= WINDOW || request.text.len() > MAX_TRAIN_BYTES {
        return Err(format!("Training text must be 9 to {MAX_TRAIN_BYTES} ASCII bytes"));
    }
    if !(1..=25).contains(&request.epochs) {
        return Err("Epoch count must be between 1 and 25".into());
    }
    if state.shared.training.compare_exchange(
        false, true, Ordering::AcqRel, Ordering::Acquire
    ).is_err() {
        return Err("Neural Nova is already training".into());
    }
    state.shared.cancel.store(false, Ordering::Release);
    let state = state.inner().clone();
    let corpus = request.text.into_bytes();
    tauri::async_runtime::spawn_blocking(move || {
        let result = train_inner(&app, &state, &corpus, request.epochs);
        state.shared.training.store(false, Ordering::Release);
        match result {
            Ok(cancelled) => {
                let _ = app.emit("neural-done", serde_json::json!({ "cancelled": cancelled }));
            }
            Err(message) => {
                let _ = app.emit("neural-error", message);
            }
        }
    });
    Ok(())
}

fn train_inner(app: &AppHandle, state: &NeuralState, corpus: &[u8], epochs: u32) -> Result<bool, String> {
    let total = (corpus.len() - WINDOW) * epochs as usize;
    let mut completed = 0usize;
    let mut cancelled = false;
    for epoch in 1..=epochs {
        let mut position = 0usize;
        let mut batch_counter = 0usize;
        let mut total_epoch_loss = 0.0_f32;
        while position + WINDOW < corpus.len() {
            if state.shared.cancel.load(Ordering::Acquire) {
                cancelled = true;
                break;
            }
            let mut storage = Vec::with_capacity(BATCH);
            for _ in 0..BATCH {
                if position + WINDOW >= corpus.len() { break; }
                let mut context = [0u8; WINDOW];
                context.copy_from_slice(&corpus[position..position + WINDOW]);
                storage.push((encode(&context), corpus[position + WINDOW] as usize));
                position += 1;
            }
            let batch: Vec<(&[f32], usize)> = storage.iter()
                .map(|(features, target)| (features.as_slice(), *target)).collect();
            let (loss, steps, observation) = {
                let mut session = lock_session(state)?;
                let s = &mut *session;
                let loss = s.network.train_batch(&batch, &mut s.optimizer)?;
                s.steps += 1;
                s.examples += batch.len() as u64;
                s.trained = true;
                s.last_loss = Some(loss);
                let context = &corpus[position - 1..position - 1 + WINDOW];
                let mut bytes = [0u8; WINDOW];
                bytes.copy_from_slice(context);
                let observation = if batch_counter % 8 == 0 {
                    Some(trace(&s.network, &bytes)?)
                } else { None };
                (loss, s.steps, observation)
            };
            batch_counter += 1;
            completed += batch.len();
            total_epoch_loss += loss;
            if let Some(trace) = observation {
                let _ = app.emit("neural-progress", TrainingProgress {
                    epoch, epochs, completed_examples: completed,
                    total_examples: total, loss, steps, trace,
                });
            }
        }
        {
            let session = lock_session(state)?;
            session.network.save(checkpoint_path(app)?)?;
        }
        let _ = app.emit("neural-epoch", serde_json::json!({
            "epoch":epoch,"epochs":epochs,
            "loss": total_epoch_loss / (batch_counter.max(1) as f32),
            "completedExamples":completed,"totalExamples":total
        }));
        if cancelled { break; }
    }
    Ok(cancelled)
}

#[tauri::command]
pub fn neural_stop(state: State<'_, NeuralState>) -> bool {
    let running = state.shared.training.load(Ordering::Acquire);
    if running { state.shared.cancel.store(true, Ordering::Release); }
    running
}

#[tauri::command]
pub fn neural_reset(app: AppHandle, state: State<'_, NeuralState>) -> Result<BrainStatus, String> {
    if state.shared.training.load(Ordering::Acquire) {
        return Err("Stop training before resetting the neural network".into());
    }
    let mut session = lock_session(&state)?;
    let network = Network::new(NetworkConfig::neural_nova_v1())?;
    session.optimizer = Adam::new(&network, 0.001)?;
    session.network = network;
    session.steps = 0;
    session.examples = 0;
    session.last_loss = None;
    session.trained = false;
    let path = checkpoint_path(&app)?;
    if path.exists() { fs::remove_file(path).map_err(|e| e.to_string())?; }
    Ok(status(&state, &session))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_ascii_architecture_input_encoding() {
        let state = NeuralState::default();
        let session = lock_session(&state).unwrap();
        assert_eq!(session.network.dimensions(), vec![1024,512,512,128]);
        let v = encode(b"abcdefgh");
        assert_eq!(v.len(), 1024);
        assert_eq!(v.iter().filter(|&&x| x == 1.0).count(), 8);
        let t = trace(&session.network, b"abcdefgh").unwrap();
        assert_eq!(t.layers.iter().map(Vec::len).collect::<Vec<_>>(), vec![1024,512,512,128]);
        assert!(t.links.iter().all(|link|
            session.network.connection_weight(link.transition,link.from,link.to) == Some(link.weight)
        ));
    }
}
