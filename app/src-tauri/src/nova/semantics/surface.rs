use super::types::{QuoteRange, SurfaceObservation, SurfaceSegment, SurfaceSentence, SurfaceToken};

const SURFACE_VERSION: &str = "7.0.0-semantic-teaching-pass-6";

fn normalize_display(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_quote_boundary_before(raw: &str, byte_index: usize) -> bool {
    if byte_index == 0 {
        return true;
    }
    raw[..byte_index]
        .chars()
        .next_back()
        .map(|c| c.is_whitespace() || "([{,:;".contains(c))
        .unwrap_or(true)
}

fn is_quote_boundary_after(raw: &str, byte_index: usize) -> bool {
    if byte_index >= raw.len() {
        return true;
    }
    raw[byte_index..]
        .chars()
        .next()
        .map(|c| c.is_whitespace() || ")]}.,!?;:".contains(c))
        .unwrap_or(true)
}

fn scan_quote_style(raw: &str, open: char, close: char, marker: &str, strict_boundary: bool) -> Vec<QuoteRange> {
    let mut out = Vec::new();
    let chars: Vec<(usize, char)> = raw.char_indices().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let (open_idx, ch) = chars[i];
        if ch != open || (strict_boundary && !is_quote_boundary_before(raw, open_idx)) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() {
            let (close_idx, candidate) = chars[j];
            if candidate == close {
                let close_end = close_idx + candidate.len_utf8();
                if strict_boundary && !is_quote_boundary_after(raw, close_end) {
                    j += 1;
                    continue;
                }
                let inner_start = open_idx + ch.len_utf8();
                if close_idx > inner_start {
                    out.push(QuoteRange {
                        id: format!("quote_{}_{}", open_idx, close_idx),
                        text: raw[inner_start..close_idx].to_string(),
                        start: inner_start,
                        end: close_idx,
                        full_start: open_idx,
                        full_end: close_end,
                        marker: marker.to_string(),
                    });
                }
                i = j;
                break;
            }
            j += 1;
        }
        i += 1;
    }
    out
}

pub fn quote_ranges(raw: &str) -> Vec<QuoteRange> {
    let mut out = Vec::new();
    out.extend(scan_quote_style(raw, '“', '”', "“”", false));
    out.extend(scan_quote_style(raw, '‘', '’', "‘’", false));
    out.extend(scan_quote_style(raw, '"', '"', "\"\"", false));
    out.extend(scan_quote_style(raw, '\'', '\'', "''", true));
    out.sort_by_key(|q| q.full_start);

    // Prefer the outermost non-overlapping surface boundaries. A later nested
    // scope system may represent true nested quotation explicitly.
    let mut filtered = Vec::new();
    let mut last_end = 0usize;
    for q in out {
        if !filtered.is_empty() && q.full_start < last_end {
            continue;
        }
        last_end = q.full_end;
        filtered.push(q);
    }
    filtered
}

fn surface_segments(raw: &str, quotes: &[QuoteRange]) -> Vec<SurfaceSegment> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    for quote in quotes {
        if quote.full_start > pos {
            out.push(SurfaceSegment {
                kind: "outer_text".to_string(),
                text: raw[pos..quote.full_start].to_string(),
                start: pos,
                end: quote.full_start,
                quote_id: None,
            });
        }
        out.push(SurfaceSegment {
            kind: "quoted_text".to_string(),
            text: quote.text.clone(),
            start: quote.start,
            end: quote.end,
            quote_id: Some(quote.id.clone()),
        });
        pos = quote.full_end;
    }
    if pos < raw.len() {
        out.push(SurfaceSegment {
            kind: "outer_text".to_string(),
            text: raw[pos..].to_string(),
            start: pos,
            end: raw.len(),
            quote_id: None,
        });
    }
    out.into_iter().filter(|s| !s.text.is_empty()).collect()
}

fn sentences_in_segment(segment: &SurfaceSegment) -> Vec<SurfaceSentence> {
    let mut out = Vec::new();
    let mut start_local = 0usize;
    let mut serial = 0usize;
    for (idx, ch) in segment.text.char_indices() {
        if !matches!(ch, '.' | '!' | '?') {
            continue;
        }
        let end_local = idx + ch.len_utf8();
        let raw_piece = &segment.text[start_local..end_local];
        let leading = raw_piece.len() - raw_piece.trim_start().len();
        let trimmed = raw_piece.trim();
        if !trimmed.is_empty() {
            serial += 1;
            let absolute_start = segment.start + start_local + leading;
            out.push(SurfaceSentence {
                id: format!("sentence_{}_{}", segment.start, serial),
                text: trimmed.to_string(),
                start: absolute_start,
                end: segment.start + end_local,
                terminal: Some(ch.to_string()),
                scope_id: segment
                    .quote_id
                    .as_ref()
                    .map(|id| format!("scope_{id}"))
                    .unwrap_or_else(|| "scope_discourse".to_string()),
            });
        }
        start_local = end_local;
    }
    if start_local < segment.text.len() {
        let raw_piece = &segment.text[start_local..];
        let leading = raw_piece.len() - raw_piece.trim_start().len();
        let trimmed = raw_piece.trim();
        if !trimmed.is_empty() {
            serial += 1;
            out.push(SurfaceSentence {
                id: format!("sentence_{}_{}", segment.start, serial),
                text: trimmed.to_string(),
                start: segment.start + start_local + leading,
                end: segment.end,
                terminal: None,
                scope_id: segment
                    .quote_id
                    .as_ref()
                    .map(|id| format!("scope_{id}"))
                    .unwrap_or_else(|| "scope_discourse".to_string()),
            });
        }
    }
    out
}

fn tokens_in_segment(segment: &SurfaceSegment) -> Vec<SurfaceToken> {
    let mut out = Vec::new();
    let mut current_start: Option<usize> = None;
    let mut current = String::new();

    let flush = |out: &mut Vec<SurfaceToken>, current: &mut String, start: &mut Option<usize>, end: usize| {
        if current.is_empty() {
            *start = None;
            return;
        }
        let local_start = start.unwrap_or(0);
        let first = current.chars().next();
        out.push(SurfaceToken {
            surface: current.clone(),
            start: segment.start + local_start,
            end: segment.start + end,
            word: current.chars().any(|c| c.is_alphanumeric()),
            capitalized: first.map(|c| c.is_uppercase()).unwrap_or(false),
            scope_id: segment
                .quote_id
                .as_ref()
                .map(|id| format!("scope_{id}"))
                .unwrap_or_else(|| "scope_discourse".to_string()),
        });
        current.clear();
        *start = None;
    };

    for (idx, ch) in segment.text.char_indices() {
        let word_char = ch.is_alphanumeric() || matches!(ch, '_' | '\'' | '’' | '-');
        if word_char {
            if current_start.is_none() {
                current_start = Some(idx);
            }
            current.push(ch);
        } else {
            flush(&mut out, &mut current, &mut current_start, idx);
            if !ch.is_whitespace() {
                out.push(SurfaceToken {
                    surface: ch.to_string(),
                    start: segment.start + idx,
                    end: segment.start + idx + ch.len_utf8(),
                    word: false,
                    capitalized: false,
                    scope_id: segment
                        .quote_id
                        .as_ref()
                        .map(|id| format!("scope_{id}"))
                        .unwrap_or_else(|| "scope_discourse".to_string()),
                });
            }
        }
    }
    flush(
        &mut out,
        &mut current,
        &mut current_start,
        segment.text.len(),
    );
    out
}

pub fn observe(raw: &str) -> SurfaceObservation {
    let quotes = quote_ranges(raw);
    let segments = surface_segments(raw, &quotes);
    let mut sentences = Vec::new();
    let mut tokens = Vec::new();
    for segment in &segments {
        sentences.extend(sentences_in_segment(segment));
        tokens.extend(tokens_in_segment(segment));
    }
    sentences.sort_by_key(|s| s.start);
    tokens.sort_by_key(|t| t.start);

    SurfaceObservation {
        version: SURFACE_VERSION.to_string(),
        raw: raw.to_string(),
        normalized_for_display: normalize_display(raw),
        quotes,
        segments,
        sentences,
        tokens,
        note: "Rust-owned surface evidence only. Quotation boundaries, tokens, and sentence boundaries do not assign semantic meaning by themselves.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_curly_single_quotes_as_real_surface_boundaries() {
        let obs = observe("In ‘Mira opened the box. Who opened the box?’, what relation is requested?");
        assert_eq!(obs.quotes.len(), 1);
        assert_eq!(obs.quotes[0].text, "Mira opened the box. Who opened the box?");
        assert!(obs.sentences.iter().any(|s| s.scope_id != "scope_discourse" && s.terminal.as_deref() == Some("?")));
        assert!(obs.sentences.iter().any(|s| s.scope_id == "scope_discourse" && s.terminal.as_deref() == Some("?")));
    }
}
