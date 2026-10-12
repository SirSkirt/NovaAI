//! Tiny proof-of-concept character model: 8 ASCII characters -> next ASCII character.
//! An 8 x 128 one-hot input provides exactly 1,024 input features.
//! This is a feed-forward character predictor, NOT yet a Transformer LLM.

use neural_nova::{Adam, Network, NetworkConfig};
use std::env;

const ALPHABET: usize = 128;
const CONTEXT: usize = 8;
const BATCH_SIZE: usize = 16;

fn encode(window: &[u8]) -> Vec<f32> {
    assert_eq!(window.len(), CONTEXT);
    let mut features = vec![0.0; CONTEXT * ALPHABET];
    for (position, &byte) in window.iter().enumerate() {
        features[position * ALPHABET + byte as usize] = 1.0;
    }
    features
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --release --bin train_ascii -- <ascii-corpus.txt> [epochs] [checkpoint.nnov]");
        return Err("Missing ASCII training corpus".into());
    }
    let epochs = args.get(2).map(|s| s.parse::<usize>())
        .transpose().map_err(|e| e.to_string())?.unwrap_or(2);
    if !(1..=100).contains(&epochs) { return Err("Epochs must be between 1 and 100".into()); }
    let checkpoint = args.get(3).map(String::as_str).unwrap_or("neural-nova.nnov");
    let corpus = std::fs::read(&args[1]).map_err(|e| e.to_string())?;
    if corpus.len() <= CONTEXT { return Err("Corpus must exceed 8 bytes".into()); }
    if !corpus.is_ascii() {
        return Err("First prototype accepts ASCII only; use an ASCII corpus".into());
    }
    let mut network = Network::new(NetworkConfig::neural_nova_v1())?;
    let mut optimizer = Adam::new(&network, 0.001)?;
    println!("Neural Nova: {:?} ({} trainable parameters)",
             network.dimensions(), network.parameter_count());

    for epoch in 0..epochs {
        let mut running_loss = 0.0;
        let mut batches = 0;
        let mut cursor = 0;
        while cursor + CONTEXT < corpus.len() {
            let mut owned: Vec<(Vec<f32>, usize)> = Vec::with_capacity(BATCH_SIZE);
            for _ in 0..BATCH_SIZE {
                if cursor + CONTEXT >= corpus.len() { break; }
                let features = encode(&corpus[cursor..cursor + CONTEXT]);
                let target = corpus[cursor + CONTEXT] as usize;
                owned.push((features, target));
                cursor += 1;
            }
            let references: Vec<(&[f32], usize)> = owned.iter()
                .map(|(features, target)| (features.as_slice(), *target))
                .collect();
            running_loss += network.train_batch(&references, &mut optimizer)?;
            batches += 1;
            if batches % 100 == 0 {
                println!("epoch={} batch={} mean_loss={:.4}",
                         epoch + 1, batches, running_loss / batches as f32);
            }
        }
        println!("epoch={} mean_loss={:.4}",
                 epoch + 1, running_loss / batches as f32);
        network.save(checkpoint)?;
        println!("saved checkpoint: {checkpoint}");
    }
    Ok(())
}