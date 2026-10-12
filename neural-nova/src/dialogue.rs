//! Supervised dialogue curriculum. All answers are learned via gradients;
//! no rule-based replies or string lookup are used at inference.
use crate::{encode_ascii_context, Network};

pub const MAX_DIALOGUE_PAIRS: usize = 1024;
pub const MAX_QUESTION_BYTES: usize = 180;
pub const MAX_ANSWER_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialoguePair {
    pub user: String,
    pub assistant: String,
}

pub fn parse_dialogues(text: &str) -> Result<Vec<DialoguePair>, String> {
    let mut pairs = Vec::new();
    for (line, raw) in text.lines().enumerate() {
        let raw = raw.trim_end_matches('\r');
        if raw.trim().is_empty() || raw.starts_with('#') { continue; }
        let (question, answer) = raw.split_once('\t')
            .ok_or_else(|| format!("Line {}: expected question<TAB>answer", line + 1))?;
        if answer.contains('\t') {
            return Err(format!("Line {}: only one TAB separator is allowed", line + 1));
        }
        let pair = DialoguePair {
            user: question.trim().to_string(),
            assistant: answer.trim().to_string(),
        };
        validate_pair(&pair).map_err(|error| format!("Line {}: {error}",line + 1))?;
        if pairs.iter().any(|previous: &DialoguePair| previous == &pair) {
            continue;
        }
        pairs.push(pair);
        if pairs.len() > MAX_DIALOGUE_PAIRS {
            return Err(format!("Maximum {MAX_DIALOGUE_PAIRS} unique dialogue examples per run"));
        }
    }
    if pairs.is_empty() { return Err("No dialogue examples supplied".into()); }
    Ok(pairs)
}

pub fn validate_pair(pair: &DialoguePair) -> Result<(), String> {
    if pair.user.is_empty() || pair.assistant.is_empty() {
        return Err("Questions and responses must not be empty".into());
    }
    if !pair.user.is_ascii() || !pair.assistant.is_ascii() {
        return Err("Only ASCII text is supported in this experimental version".into());
    }
    if pair.user.len() > MAX_QUESTION_BYTES || pair.assistant.len() > MAX_ANSWER_BYTES {
        return Err(format!("Question max {MAX_QUESTION_BYTES} bytes; answer max {MAX_ANSWER_BYTES} bytes"));
    }
    if pair.user.contains('\n') || pair.user.contains('\r') ||
       pair.assistant.contains('\n') || pair.assistant.contains('\r') {
        return Err("Each question and answer must be one line".into());
    }
    Ok(())
}

/// Short role markers conserve the 64-character context budget.
pub fn dialogue_prefix(question: &str) -> Result<Vec<u8>, String> {
    let pair = DialoguePair {
        user: question.trim().to_string(),
        assistant: ".".to_string(),
    };
    validate_pair(&pair)?;
    Ok(format!("U: {}\nN: ", pair.user).into_bytes())
}

/// Only answer tokens contribute to the supervised loss. The question is
/// conditioning context, not a memorization target. A newline ends the reply.
pub fn supervised_samples(pair: &DialoguePair) -> Result<Vec<(Vec<f32>, usize)>, String> {
    validate_pair(pair)?;
    let mut history = dialogue_prefix(&pair.user)?;
    let mut samples = Vec::with_capacity(pair.assistant.len() + 1);
    for byte in pair.assistant.bytes().chain(std::iter::once(b'\n')) {
        samples.push((encode_ascii_context(&history)?, byte as usize));
        history.push(byte);
    }
    Ok(samples)
}

/// True held-out evaluation: entire distinct question/answer pairs are reserved.
/// Answer-only cross-entropy and exact next-character accuracy; they cannot by
/// themselves establish semantic generalization or conversational competence.
pub fn evaluate_dialogues(
    network: &Network,
    pairs: &[DialoguePair],
) -> Result<(f32, f32), String> {
    let mut loss = 0.0f64;
    let mut correct = 0usize;
    let mut count = 0usize;
    for pair in pairs {
        for (features, target) in supervised_samples(pair)? {
            let p = network.predict(&features)?;
            loss -= (p[target] as f64).max(1e-30).ln();
            let prediction = p.iter().enumerate().max_by(|a,b|
                a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
            ).map(|(index,_)| index).unwrap_or(0);
            correct += usize::from(prediction == target);
            count += 1;
        }
    }
    if count == 0 { return Err("Cannot evaluate empty dialogue dataset".into()); }
    Ok(((loss / count as f64) as f32, correct as f32 / count as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NetworkConfig, Network};
    #[test]
    fn split_on_tabs_and_dedupe_without_inventing_replies() {
        let data="hello\tHi!\nwhy?\tBecause of the rain.\nhello\tHi!\n";
        let pairs=parse_dialogues(data).unwrap();
        assert_eq!(pairs.len(),2);
        assert_eq!(pairs[1].assistant,"Because of the rain.");
        assert!(parse_dialogues("hello|hi").is_err());
        assert!(parse_dialogues("a\tcafé").is_err());
    }

    #[test]
    fn response_only_supervision_and_end_marker() {
        let pair=DialoguePair{user:"Hello Nova".into(),assistant:"Hi!".into()};
        let samples=supervised_samples(&pair).unwrap();
        assert_eq!(samples.len(),4); // 3 reply chars + newline terminator
        assert_eq!(samples.iter().map(|x|x.1).collect::<Vec<_>>(),
            vec![b'H' as usize,b'i' as usize,b'!' as usize,b'\n' as usize]);
        assert_eq!(samples[0].0.len(),1024);
        assert_ne!(samples[0].0,samples[1].0);
        assert!(String::from_utf8(dialogue_prefix("Hello Nova").unwrap()).unwrap().ends_with("\nN: "));
    }

    #[test]
    fn holdout_metrics_are_finite() {
        let network=Network::new(NetworkConfig{dimensions:vec![1024,24,128],seed:4}).unwrap();
        let pairs=parse_dialogues("hello\tHello!\nbye\tGoodbye.\n").unwrap();
        let (loss,accuracy)=evaluate_dialogues(&network,&pairs).unwrap();
        assert!(loss.is_finite() && loss>0.0);
        assert!((0.0..=1.0).contains(&accuracy));
    }
    #[test]
    fn response_supervision_really_updates_neural_weights() {
        use crate::Adam;
        let pair=DialoguePair{user:"hello nova".into(),assistant:"Hi!".into()};
        let samples=supervised_samples(&pair).unwrap();
        let mut network=Network::new(NetworkConfig{
            dimensions:vec![1024,24,128],seed:37
        }).unwrap();
        let mut optimizer=Adam::new(&network,0.008).unwrap();
        let mut before=0.0f32;
        for (feature,target) in &samples {
            before+=-(network.predict(feature).unwrap()[*target].max(1e-30)).ln();
        }
        for _ in 0..95 {
            let refs:Vec<(&[f32],usize)>=samples.iter()
                .map(|(feature,target)|(feature.as_slice(),*target)).collect();
            network.train_batch(&refs,&mut optimizer).unwrap();
        }
        let mut after=0.0f32;
        for (feature,target) in &samples {
            after+=-(network.predict(feature).unwrap()[*target].max(1e-30)).ln();
        }
        assert!(after<before*0.2,"Training loss should fall: {before} -> {after}");
    }

}
