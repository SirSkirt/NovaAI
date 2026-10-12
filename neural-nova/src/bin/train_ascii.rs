//! Command-line character prediction experiment using the same
//! 64-character / 1024-feature input encoder as Neural Nova's GUI.
//! The output remains 128 ASCII character probabilities. No external ML crate.
use neural_nova::{encode_ascii_context, Adam, Network, NetworkConfig, CONTEXT_CHARS};
use std::env;

const BATCH_SIZE: usize = 8;

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --release --bin train_ascii -- <ascii-corpus.txt> [epochs] [checkpoint.nnov]");
        return Err("Missing ASCII training corpus".into());
    }
    let epochs = args.get(2).map(|s| s.parse::<usize>())
        .transpose().map_err(|e| e.to_string())?.unwrap_or(2);
    if !(1..=100).contains(&epochs) { return Err("Epochs must be between 1 and 100".into()); }
    let checkpoint = args.get(3).map(String::as_str).unwrap_or("neural-nova-v0.1.2-cli.nnov");
    let corpus = std::fs::read(&args[1]).map_err(|e| e.to_string())?;
    if corpus.len() < 40 { return Err("Corpus needs at least 40 bytes".into()); }
    if !corpus.is_ascii() { return Err("First-stage language model accepts ASCII text only".into()); }

    let mut network = Network::new(NetworkConfig::neural_nova_v1())?;
    let mut optimizer = Adam::new(&network, 0.001)?;
    let train_end = corpus.len() * 9 / 10;
    println!("Neural Nova v0.1.2: {:?} ({} parameters, {} context chars)",
             network.dimensions(), network.parameter_count(), CONTEXT_CHARS);
    for epoch in 0..epochs {
        let mut sum_loss = 0.0f32;
        let mut batches = 0usize;
        for block in (0..train_end).collect::<Vec<_>>().chunks(BATCH_SIZE) {
            let mut owned: Vec<(Vec<f32>, usize)> = Vec::with_capacity(BATCH_SIZE);
            for &index in block {
                let features = encode_ascii_context(
                    &corpus[index.saturating_sub(CONTEXT_CHARS)..index]
                )?;
                owned.push((features, corpus[index] as usize));
            }
            let references: Vec<(&[f32], usize)> = owned.iter()
                .map(|(features, target)| (features.as_slice(), *target))
                .collect();
            sum_loss += network.train_batch(&references, &mut optimizer)?;
            batches += 1;
        }
        let mut validation_loss = 0.0;
        for position in train_end..corpus.len() {
            let probabilities = network.predict(&encode_ascii_context(
                &corpus[position.saturating_sub(CONTEXT_CHARS)..position]
            )?)?;
            validation_loss -= probabilities[corpus[position] as usize].max(1e-30).ln();
        }
        println!("epoch={} train_loss={:.4} validation_loss={:.4}",
                 epoch+1, sum_loss/batches as f32,
                 validation_loss/(corpus.len()-train_end) as f32);
        network.save(checkpoint)?;
    }
    println!("saved checkpoint: {checkpoint}");
    Ok(())
}
