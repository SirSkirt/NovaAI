use neural_nova::{encode_ascii_context, Adam, Network, NetworkConfig, CONTEXT_CHARS};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering as CompareOrdering;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const WINDOW: usize = CONTEXT_CHARS;
const ALPHABET: usize = 128;
const BATCH: usize = 8;
const MAX_TRAIN_BYTES: usize = 250_000;
// New context encoding must not silently interpret legacy eight-char weights.
const CHECKPOINT: &str = "neural-nova-v0.1.2.nnov";

pub struct BrainSession {
    network: Network,
    optimizer: Adam,
    steps: u64,
    examples: u64,
    last_loss: Option<f32>,
    trained: bool,
    completed_examples: usize,
    total_examples: usize,
    current_epoch: u32,
    total_epochs: u32,
    last_error: Option<String>,
    validation_loss: Option<f32>,
    validation_accuracy: Option<f32>,
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
                    completed_examples: 0,
                    total_examples: 0,
                    current_epoch: 0,
                    total_epochs: 0,
                    last_error: None,
                    validation_loss: None,
                    validation_accuracy: None,
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
    completed_examples: usize,
    total_examples: usize,
    current_epoch: u32,
    total_epochs: u32,
    last_error: Option<String>,
    validation_loss: Option<f32>,
    validation_accuracy: Option<f32>,
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
        completed_examples: session.completed_examples,
        total_examples: session.total_examples,
        current_epoch: session.current_epoch,
        total_epochs: session.total_epochs,
        last_error: session.last_error.clone(),
        validation_loss: session.validation_loss,
        validation_accuracy: session.validation_accuracy,
    }
}

fn context_bytes(input: &str) -> Result<Vec<u8>, String> {
    if !input.is_ascii() {
        return Err("This 128-output character model currently supports ASCII only".into());
    }
    let bytes = input.as_bytes();
    Ok(bytes[bytes.len().saturating_sub(WINDOW)..].to_vec())
}

/// Every example has a real preceding context, optionally shortened for
/// prefix generalization. Training and inference use the same feature encoder.
fn features_for(corpus: &[u8], position: usize, window: usize) -> Result<Vec<f32>, String> {
    let start = position.saturating_sub(window.min(WINDOW));
    encode_ascii_context(&corpus[start..position])
}

fn validation_metrics(network: &Network, corpus: &[u8], start: usize) -> Result<(f32, f32), String> {
    let mut loss = 0.0f64;
    let mut correct = 0usize;
    let count = corpus.len() - start;
    for index in start..corpus.len() {
        let probabilities = network.predict(&features_for(corpus, index, WINDOW)?)?;
        let target = corpus[index] as usize;
        loss += -(probabilities[target].max(1e-30) as f64).ln();
        let predicted = probabilities.iter().enumerate().max_by(|a,b|
            a.1.partial_cmp(b.1).unwrap_or(CompareOrdering::Equal)
        ).map(|v|v.0).unwrap_or(0);
        correct += usize::from(predicted == target);
    }
    Ok(((loss / count as f64) as f32, correct as f32 / count as f32))
}

fn randomized_positions(length: usize, epoch: u32) -> Vec<usize> {
    let mut positions: Vec<usize> = (0..length).collect();
    let mut seed = 0x4e4f_5641_2026_1002u64 ^ (epoch as u64).wrapping_mul(0x9e3779b97f4a7c15);
    for i in (1..length).rev() {
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        let random = seed.wrapping_mul(2685821657736338717);
        positions.swap(i, (random % (i as u64 + 1)) as usize);
    }
    positions
}

fn best_indices(values: &[f32], count: usize) -> Vec<usize> {
    let mut ids: Vec<usize> = (0..values.len()).filter(|&idx| values[idx] > 0.0).collect();
    ids.sort_unstable_by(|&a, &b| {
        values[b].partial_cmp(&values[a]).unwrap_or(CompareOrdering::Equal)
    });
    ids.truncate(count);
    ids
}

fn trace(network: &Network, context: &[u8]) -> Result<BrainTrace, String> {
    let layers = network.activations(&encode_ascii_context(context)?)?;
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
            let probabilities = session.network.predict(&encode_ascii_context(&context)?)?;
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
    if request.text.len() < 40 || request.text.len() > MAX_TRAIN_BYTES {
        return Err(format!("Training text must be 40 to {MAX_TRAIN_BYTES} ASCII bytes"));
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
    // Hold out the final 10% of the corpus to report generalization, not just memorization.
    let train_count = ((request.text.len() * 9) / 10).max(1);
    let total_examples = train_count * request.epochs as usize;
    {
        let setup = (|| -> Result<(), String> {
            let mut session = lock_session(&state)?;
            session.completed_examples = 0;
            session.total_examples = total_examples;
            session.current_epoch = 1;
            session.total_epochs = request.epochs;
            session.last_error = None;
            session.validation_loss = None;
            session.validation_accuracy = None;
            Ok(())
        })();
        if let Err(error) = setup {
            state.shared.training.store(false, Ordering::Release);
            return Err(error);
        }
    }
    let state = state.inner().clone();
    let corpus = request.text.into_bytes();
    tauri::async_runtime::spawn_blocking(move || {
        let result = train_inner(&app, &state, &corpus, request.epochs);
        if let Err(ref message) = result {
            if let Ok(mut session) = lock_session(&state) {
                session.last_error = Some(message.clone());
            }
            eprintln!("Neural Nova training worker error: {message}");
        }
        state.shared.training.store(false, Ordering::Release);
        match result {
            Ok(cancelled) => {
                if let Err(error) = app.emit("neural-done", serde_json::json!({ "cancelled": cancelled })) {
                    eprintln!("Neural Nova could not emit training completion: {error}");
                }
            }
            Err(message) => {
                if let Err(error) = app.emit("neural-error", message) {
                    eprintln!("Neural Nova could not emit training error: {error}");
                }
            }
        }
    });
    Ok(())
}

fn train_inner(app: &AppHandle, state: &NeuralState, corpus: &[u8], epochs: u32) -> Result<bool, String> {
    let training_count = corpus.len() * 9 / 10;
    let total = training_count * epochs as usize;
    let mut completed = 0usize;
    let mut cancelled = false;
    for epoch in 1..=epochs {
        let positions = randomized_positions(training_count, epoch);
        let mut batch_counter = 0usize;
        let mut total_epoch_loss = 0.0_f64;
        for batch_indices in positions.chunks(BATCH) {
            if state.shared.cancel.load(Ordering::Acquire) {
                cancelled = true;
                break;
            }
            let mut storage = Vec::with_capacity(batch_indices.len());
            for &index in batch_indices {
                // Some training examples deliberately contain only a short prefix.
                // Others include the full 64 characters. This makes short prompts
                // and longer sentence contexts both useful with the same weights.
                let length = match (index.wrapping_mul(13) + epoch as usize) % 5 {
                    0 => 8,
                    1 => 16,
                    2 => 32,
                    _ => WINDOW,
                };
                storage.push((features_for(corpus, index, length)?, corpus[index] as usize));
            }
            let batch: Vec<(&[f32], usize)> = storage.iter()
                .map(|(features, target)| (features.as_slice(), *target)).collect();
            let (loss, steps, observation) = {
                let mut session = lock_session(state)?;
                let brain = &mut *session;
                let loss = brain.network.train_batch(&batch, &mut brain.optimizer)?;
                brain.steps += 1;
                brain.examples += batch.len() as u64;
                brain.completed_examples = completed + batch.len();
                brain.current_epoch = epoch;
                brain.trained = true;
                brain.last_loss = Some(loss);
                // Visualize a representative training sample rather than an
                // assumption that shuffled sample indices are contiguous.
                let observation = if batch_counter % 8 == 0 {
                    let index = batch_indices[0];
                    Some(trace(&brain.network, &corpus[index.saturating_sub(WINDOW)..index])?)
                } else { None };
                (loss, brain.steps, observation)
            };
            batch_counter += 1;
            completed += batch.len();
            total_epoch_loss += loss as f64;
            if let Some(trace) = observation {
                if let Err(error) = app.emit("neural-progress", TrainingProgress {
                    epoch, epochs, completed_examples: completed,
                    total_examples: total, loss, steps, trace,
                }) {
                    eprintln!("Unable to emit neural-progress: {error}");
                }
            }
        }
        // Save only the new encoding's weights. The original v0.1 checkpoint
        // remains untouched for reproducibility of earlier experiments.
        let (heldout_loss, heldout_accuracy, mean_loss) = {
            let mut session = lock_session(state)?;
            let (heldout_loss, heldout_accuracy) =
                validation_metrics(&session.network, corpus, training_count)?;
            session.validation_loss = Some(heldout_loss);
            session.validation_accuracy = Some(heldout_accuracy);
            session.network.save(checkpoint_path(app)?)?;
            (
                heldout_loss,
                heldout_accuracy,
                total_epoch_loss / batch_counter.max(1) as f64,
            )
        };
        if let Err(error) = app.emit("neural-epoch", serde_json::json!({
            "epoch":epoch,"epochs":epochs,
            "loss":mean_loss,
            "validationLoss":heldout_loss,
            "validationAccuracy":heldout_accuracy,
            "completedExamples":completed,"totalExamples":total
        })) { eprintln!("Unable to emit neural-epoch: {error}"); }
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
    session.completed_examples = 0;
    session.total_examples = 0;
    session.current_epoch = 0;
    session.total_epochs = 0;
    session.last_error = None;
    session.validation_loss = None;
    session.validation_accuracy = None;
    let path = checkpoint_path(&app)?;
    if path.exists() { fs::remove_file(path).map_err(|e| e.to_string())?; }
    Ok(status(&state, &session))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_context_uses_exact_dimensions_and_real_weights() {
        let state = NeuralState::default();
        let session = lock_session(&state).unwrap();
        assert_eq!(session.network.dimensions(), vec![1024,512,512,128]);
        let v = encode_ascii_context(b"the curious little cat followed the mouse").unwrap();
        assert_eq!(v.len(), 1024);
        let trace = trace(&session.network, b"the curious little cat followed the mouse").unwrap();
        assert_eq!(trace.layers.iter().map(Vec::len).collect::<Vec<_>>(), vec![1024,512,512,128]);
        assert!(trace.links.iter().all(|link|
            session.network.connection_weight(link.transition,link.from,link.to) == Some(link.weight)
        ));
    }

    #[test]
    fn deterministic_holdout_is_never_in_training_indices() {
        let corpus = b"The quick brown fox jumped over the old wooden fence. A fox can run quickly.";
        let split = corpus.len()*9/10;
        let indices = randomized_positions(split, 4);
        assert_eq!(indices.len(), split);
        assert!(indices.iter().all(|&i|i<split));
        let model = Network::new(NetworkConfig { dimensions: vec![1024,16,128], seed:5 }).unwrap();
        let (loss,accuracy) = validation_metrics(&model,corpus,split).unwrap();
        assert!(loss.is_finite() && (0.0..=1.0).contains(&accuracy));
    }
}
