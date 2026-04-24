use std::collections::HashMap;
use std::sync::OnceLock;

use anyhow::{Result, anyhow, bail};

const WORDS_TEXT: &str = include_str!("words.txt");

static WORD_LIST: OnceLock<Vec<&'static str>> = OnceLock::new();
static WORD_INDEX: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();

pub(super) fn word_list() -> &'static Vec<&'static str> {
    WORD_LIST.get_or_init(|| WORDS_TEXT.lines().filter(|line| !line.is_empty()).collect())
}

pub(super) fn lookup_word_indices(words: &str, expected_len: usize) -> Result<Vec<u16>> {
    let values = words
        .split_whitespace()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if values.len() != expected_len {
        bail!(
            "tor invite words must contain exactly {expected_len} words, got {}",
            values.len()
        );
    }

    values
        .iter()
        .enumerate()
        .map(|(index, word)| {
            word_index().get(word.as_str()).copied().ok_or_else(|| {
                anyhow!(
                    "unknown tor invite word at position {}: {}",
                    index + 1,
                    word
                )
            })
        })
        .collect()
}

pub(super) fn normalize_word(word: &str) -> Result<String> {
    let value = word.trim().to_ascii_lowercase();
    if value.is_empty() {
        bail!("tor invite word is empty");
    }
    if word_index().contains_key(value.as_str()) {
        return Ok(value);
    }
    bail!("unknown tor invite word: {value}");
}

fn word_index() -> &'static HashMap<&'static str, u16> {
    WORD_INDEX.get_or_init(|| {
        word_list()
            .iter()
            .enumerate()
            .map(|(index, word)| {
                let index = u16::try_from(index).expect("word list length should fit in u16");
                (*word, index)
            })
            .collect()
    })
}
