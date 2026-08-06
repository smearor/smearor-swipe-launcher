use std::collections::HashMap;

/// Trie node for German noun-stem lookup.
#[derive(Debug, Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_word_end: bool,
}

/// Trie holding common German noun stems for compound-word decomposition.
pub struct CompoundTrie {
    root: TrieNode,
}

impl CompoundTrie {
    /// Load a `CompoundTrie` from a stems file at compile time.
    ///
    /// Each line in the file is treated as one stem. Both the original
    /// capitalization and a lowercase variant are inserted so the trie
    /// matches compounds regardless of input casing.
    pub fn new(stems_text: &str) -> Self {
        let mut trie = CompoundTrie { root: TrieNode::default() };

        for line in stems_text.lines() {
            let stem = line.trim();
            if stem.is_empty() {
                continue;
            }
            trie.insert(stem);
            let lower = stem.to_lowercase();
            trie.insert(&lower);
        }

        trie
    }

    /// Insert a word into the trie.
    fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        for ch in word.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_word_end = true;
    }

    /// Check if a word exists in the trie.
    fn contains(&self, word: &str) -> bool {
        let mut node = &self.root;
        for ch in word.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return false,
            }
        }
        node.is_word_end
    }

    /// Attempt to decompose a German compound word into known stems.
    ///
    /// Uses a greedy longest-match approach: starting from the beginning,
    /// finds the longest known stem, then continues with the remainder.
    /// Returns `Some(parts)` if the word can be fully decomposed into
    /// at least two known stems, `None` otherwise.
    ///
    /// Handles the German linking morpheme "s" (Fugen-s) between stems:
    /// "Arbeitsschutz" → "Arbeit" + "s" (skip) + "Schutz".
    /// Also handles "n" and "e" linking morphemes.
    ///
    /// Trailing inflectional endings (-en, -er, -e, -es, -em, -n, -s) are
    /// accepted as valid word-final suffixes so that inflected forms like
    /// "Tiefstwerten" decompose into "Tiefst" + "Wert" + "en".
    pub fn decompose(&self, word: &str) -> Option<Vec<String>> {
        if word.len() < 6 {
            return None;
        }

        let chars: Vec<char> = word.chars().collect();
        let mut parts = Vec::new();
        let mut pos = 0;

        while pos < chars.len() {
            let remaining = &chars[pos..];
            let longest = self.find_longest_stem(remaining);

            if let Some(stem_len) = longest {
                let stem: String = remaining[..stem_len].iter().collect();
                parts.push(stem);
                pos += stem_len;

                if pos < chars.len() {
                    let next = chars[pos];
                    if next == 's' || next == 'n' || next == 'e' {
                        let after_link = &chars[pos + 1..];
                        if !after_link.is_empty() && self.find_longest_stem(after_link).is_some() {
                            pos += 1;
                        }
                    }
                }
            } else {
                let remaining_str: String = remaining.iter().collect();
                if parts.len() >= 2 && Self::is_inflectional_ending(&remaining_str) {
                    if let Some(last) = parts.last_mut() {
                        last.push_str(&remaining_str);
                    }
                    break;
                }
                return None;
            }
        }

        if parts.len() >= 2 { Some(parts) } else { None }
    }

    /// Check if `suffix` is a common German noun inflectional ending
    /// or noun-building suffix.
    fn is_inflectional_ending(suffix: &str) -> bool {
        matches!(suffix, "en" | "er" | "e" | "es" | "em" | "n" | "s" | "keit" | "heit" | "ung" | "ig")
    }

    /// Find the longest known stem starting at the beginning of `chars`.
    /// Returns the length (in chars) of the longest match, or `None`.
    fn find_longest_stem(&self, chars: &[char]) -> Option<usize> {
        let mut node = &self.root;
        let mut longest: Option<usize> = None;

        for (i, &ch) in chars.iter().enumerate() {
            match node.children.get(&ch) {
                Some(child) => {
                    node = child;
                    if node.is_word_end {
                        longest = Some(i + 1);
                    }
                }
                None => break,
            }
        }

        longest
    }
}

/// German noun stems for compound-word decomposition, loaded at compile time.
const GERMAN_STEMS_TEXT: &str = include_str!("../data/german_stems.txt");

/// Build a `CompoundTrie` preloaded with German noun stems from
/// `data/german_stems.txt`.
pub fn build_german_compound_trie() -> CompoundTrie {
    CompoundTrie::new(GERMAN_STEMS_TEXT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_luftqualitaet() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Luftqualität");
        assert!(result.is_some(), "Luftqualität should decompose");
        let parts = result.unwrap();
        assert!(parts.len() >= 2, "Should have at least 2 parts");
        assert_eq!(parts[0], "Luft");
    }

    #[test]
    fn test_decompose_luftfeuchtigkeit() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Luftfeuchtigkeit");
        assert!(result.is_some(), "Luftfeuchtigkeit should decompose");
        let parts = result.unwrap();
        assert!(parts.len() >= 2, "Should have at least 2 parts");
        assert_eq!(parts[0], "Luft");
    }

    #[test]
    fn test_decompose_arbeitsschutz() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Arbeitsschutz");
        assert!(result.is_some(), "Arbeitsschutz should decompose (Fugen-s)");
        let parts = result.unwrap();
        assert!(parts.len() >= 2, "Should have at least 2 parts");
        assert_eq!(parts[0], "Arbeit");
    }

    #[test]
    fn test_decompose_wetterbericht() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Wetterbericht");
        assert!(result.is_some(), "Wetterbericht should decompose");
    }

    #[test]
    fn test_decompose_short_word() {
        let trie = build_german_compound_trie();
        assert!(trie.decompose("Haus").is_none(), "Short words should not decompose");
    }

    #[test]
    fn test_decompose_unknown_word() {
        let trie = build_german_compound_trie();
        assert!(trie.decompose("Xyzabc").is_none(), "Unknown words should not decompose");
    }

    #[test]
    fn test_decompose_single_stem() {
        let trie = build_german_compound_trie();
        assert!(trie.decompose("Wetter").is_none(), "Single stem should not decompose");
    }

    #[test]
    fn test_decompose_temperaturmessung() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Temperaturmessung");
        assert!(result.is_some(), "Temperaturmessung should decompose");
        let parts = result.unwrap();
        assert_eq!(parts[0], "Temperatur");
    }

    #[test]
    fn test_decompose_tiefstwerten() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Tiefstwerten");
        assert!(result.is_some(), "Tiefstwerten should decompose with inflectional ending");
        let parts = result.unwrap();
        assert!(parts.contains(&"Tiefst".to_string()), "Should contain Tiefst");
        assert!(parts.contains(&"werten".to_string()), "Should contain werten (with inflectional ending)");
    }

    #[test]
    fn test_decompose_hoechstwerten() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Höchstwerten");
        assert!(result.is_some(), "Höchstwerten should decompose with inflectional ending");
        let parts = result.unwrap();
        assert!(parts.contains(&"Höchst".to_string()), "Should contain Höchst");
    }

    #[test]
    fn test_decompose_regenwahrscheinlichkeit() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Regenwahrscheinlichkeit");
        assert!(result.is_some(), "Regenwahrscheinlichkeit should decompose");
        let parts = result.unwrap();
        assert_eq!(parts[0], "Regen");
        assert!(parts.last().unwrap().ends_with("keit"), "Last part should end with keit");
    }

    #[test]
    fn test_decompose_anwendungen() {
        let trie = build_german_compound_trie();
        let result = trie.decompose("Anwendungen");
        assert!(result.is_some(), "Anwendungen should decompose");
        let parts = result.unwrap();
        assert_eq!(parts[0], "An");
        assert_eq!(parts[1], "wendungen");
    }
}
