pub mod deepl;
pub mod llm;

use crate::utils::Subtitle;

// Whether the text ends with sentence-ending punctuation, ignoring trailing
// closing quotes and brackets (e.g. `he said "yes."` still ends a sentence)
fn ends_sentence(text: &str) -> bool {
    for c in text.trim_end().chars().rev() {
        match c {
            '"' | '\'' | '`' | '”' | '’' | '」' | '』' | '）' | ')' | ']' | '】' | '》' => {
                continue
            }
            '.' | '!' | '?' | '。' | '！' | '？' | '…' | '‥' => return true,
            _ => return false,
        }
    }
    false
}

// Whisper tends to cut audio into segments in the middle of a sentence, and
// translating such fragments separately degrades the translation quality.
// Merge every segment that does not end a sentence into the next one, so the
// translators (and LLMs in particular) see complete sentences.
pub fn merge_incomplete_sentences(subtitles: &mut Vec<Subtitle>) {
    let mut merged: Vec<Subtitle> = Vec::with_capacity(subtitles.len());
    for subtitle in subtitles.drain(..) {
        let Some(last) = merged.last_mut() else {
            merged.push(subtitle);
            continue;
        };
        if ends_sentence(&last.text) {
            merged.push(subtitle);
        } else {
            last.text.push(' ');
            last.text.push_str(subtitle.text.trim());
            last.end = subtitle.end;
        }
    }
    *subtitles = merged;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subtitles(texts: &[(&str, f32, f32)]) -> Vec<Subtitle> {
        texts
            .iter()
            .map(|(text, start, end)| Subtitle::new(*start, *end, text.to_string()))
            .collect()
    }

    fn texts(subtitles: &[Subtitle]) -> Vec<&str> {
        subtitles.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn test_merge_incomplete_sentences() {
        let mut subtitles = subtitles(&[
            ("This is the first half", 0.0, 1.0),
            ("of the sentence.", 1.0, 2.0),
            ("A complete sentence.", 2.0, 3.0),
            ("Another one!", 3.0, 4.0),
        ]);
        merge_incomplete_sentences(&mut subtitles);
        assert_eq!(
            texts(&subtitles),
            vec![
                "This is the first half of the sentence.",
                "A complete sentence.",
                "Another one!"
            ]
        );
        // The merged subtitle spans both original segments
        assert_eq!(subtitles[0].start, 0.0);
        assert_eq!(subtitles[0].end, 2.0);
    }

    #[test]
    fn test_merge_chained_fragments() {
        let mut subtitles = subtitles(&[
            ("A", 0.0, 1.0),
            ("B", 1.0, 2.0),
            ("C.", 2.0, 3.0),
            ("D", 3.0, 4.0),
            ("E.", 4.0, 5.0),
        ]);
        merge_incomplete_sentences(&mut subtitles);
        assert_eq!(texts(&subtitles), vec!["A B C.", "D E."]);
    }

    #[test]
    fn test_merge_cjk_sentences() {
        let mut subtitles = subtitles(&[
            ("これは文章の", 0.0, 1.0),
            ("前半です。", 1.0, 2.0),
            ("完全な文です", 2.0, 3.0),
            ("か?", 3.0, 4.0),
        ]);
        merge_incomplete_sentences(&mut subtitles);
        assert_eq!(
            texts(&subtitles),
            vec!["これは文章の 前半です。", "完全な文です か?"]
        );
    }

    #[test]
    fn test_keep_sentences_with_trailing_closing_punctuation() {
        let mut subtitles = subtitles(&[
            ("He said \"this ends a sentence.\"", 0.0, 1.0),
            ("and this continues", 1.0, 2.0),
            ("the next one.\"", 2.0, 3.0),
        ]);
        merge_incomplete_sentences(&mut subtitles);
        // The quote after the period does not hide the sentence end, while a
        // quote after a plain word does not end a sentence either
        assert_eq!(
            texts(&subtitles),
            vec![
                "He said \"this ends a sentence.\"",
                "and this continues the next one.\""
            ]
        );
    }

    #[test]
    fn test_merge_empty_and_incomplete_inputs() {
        let mut empty: Vec<Subtitle> = Vec::new();
        merge_incomplete_sentences(&mut empty);
        assert!(empty.is_empty());

        let mut single = subtitles(&[("not finished", 0.0, 1.0)]);
        merge_incomplete_sentences(&mut single);
        assert_eq!(texts(&single), vec!["not finished"]);

        let mut all_complete = subtitles(&[("One.", 0.0, 1.0), ("Two?", 1.0, 2.0)]);
        merge_incomplete_sentences(&mut all_complete);
        assert_eq!(texts(&all_complete), vec!["One.", "Two?"]);
    }

    #[test]
    fn test_ends_sentence() {
        for text in ["End.", "End!", "End?", "完。", "です?", "wait…", "done.)"] {
            assert!(ends_sentence(text), "{:?} should end a sentence", text);
        }
        for text in ["No end", "Continue,", "です", "mid-word", ""] {
            assert!(!ends_sentence(text), "{:?} should not end a sentence", text);
        }
    }
}
