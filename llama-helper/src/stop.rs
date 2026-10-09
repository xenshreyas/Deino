//! Stop tokens over streamed text.
//!
//! A stop token can arrive split across several decoded pieces, so
//! [`StopScanner`] holds back any tail that might be the start of one and
//! releases it only once it cannot be. That way a streamed `token` line never
//! carries any part of a stop token. When a stop token appears, the text
//! before it is released with trailing whitespace trimmed, which is what the
//! sidecar has always returned for `generate`; so trailing whitespace is held
//! back too until something else follows it.

/// What to do after a piece of generated text.
#[derive(Debug, PartialEq, Eq)]
pub enum Scan {
    /// Emit this text (possibly empty) and keep generating.
    Continue(String),
    /// Emit this final text (possibly empty) and stop: a stop token appeared.
    Stopped(String),
}

/// Finds stop tokens in text that arrives piece by piece.
#[derive(Debug, Default)]
pub struct StopScanner {
    stops: Vec<String>,
    held: String,
}

impl StopScanner {
    /// A scanner for `stops`. Empty strings are ignored: they would match
    /// before any text at all.
    pub fn new(stops: &[String]) -> Self {
        let stops = stops
            .iter()
            .filter(|stop| !stop.is_empty())
            .cloned()
            .collect();
        Self {
            stops,
            held: String::new(),
        }
    }

    /// Adds the next piece and returns what is now safe to emit.
    pub fn push(&mut self, piece: &str) -> Scan {
        self.held.push_str(piece);
        if let Some(start) = self.first_stop() {
            self.held.truncate(start);
            let before = self.held.trim_end().to_owned();
            self.held.clear();
            return Scan::Stopped(before);
        }
        let keep_from = self.unsafe_tail_start();
        let tail = self.held.split_off(keep_from);
        Scan::Continue(std::mem::replace(&mut self.held, tail))
    }

    /// Ends the text without a stop token, releasing everything held back.
    pub fn finish(self) -> String {
        self.held
    }

    /// Byte offset of the earliest complete stop token in the held text.
    fn first_stop(&self) -> Option<usize> {
        self.stops
            .iter()
            .filter_map(|stop| self.held.find(stop.as_str()))
            .min()
    }

    /// Byte offset where the text that must stay held begins: the earliest
    /// suffix that could still grow into a stop token, widened to include
    /// any whitespace right before it.
    fn unsafe_tail_start(&self) -> usize {
        let prefix_start = self
            .held
            .char_indices()
            .map(|(index, _)| index)
            .find(|&index| self.could_start_stop(index))
            .unwrap_or(self.held.len());
        self.held
            .get(..prefix_start)
            .map_or(prefix_start, |before| before.trim_end().len())
    }

    /// Whether the held text from `index` on is a proper prefix of a stop token.
    fn could_start_stop(&self, index: usize) -> bool {
        let Some(suffix) = self.held.get(index..) else {
            return false;
        };
        self.stops
            .iter()
            .any(|stop| stop.len() > suffix.len() && stop.starts_with(suffix))
    }
}

#[cfg(test)]
mod tests {
    use super::{Scan, StopScanner};

    fn scanner(stops: &[&str]) -> StopScanner {
        let stops: Vec<String> = stops.iter().map(|&stop| stop.to_owned()).collect();
        StopScanner::new(&stops)
    }

    fn go(text: &str) -> Scan {
        Scan::Continue(text.to_owned())
    }

    #[test]
    fn passes_text_through_without_stop_tokens() {
        let mut scan = scanner(&[]);
        assert_eq!(scan.push("Hello"), go("Hello"));
        assert_eq!(scan.push(" world"), go(" world"));
        assert_eq!(scan.finish(), "");
    }

    #[test]
    fn holds_back_a_possible_prefix_and_releases_it_when_it_is_not_one() {
        let mut scan = scanner(&["<end_of_turn>"]);
        assert_eq!(scan.push("a <end"), go("a"));
        assert_eq!(scan.push("ing>"), go(" <ending>"));
    }

    #[test]
    fn strips_a_stop_token_split_across_pieces() {
        let mut scan = scanner(&["<end_of_turn>"]);
        assert_eq!(scan.push("Done."), go("Done."));
        assert_eq!(scan.push("<end_"), go(""));
        assert_eq!(scan.push("of_turn>"), Scan::Stopped(String::new()));
    }

    #[test]
    fn trims_whitespace_before_the_stop_token_like_legacy_generate() {
        let mut scan = scanner(&["<|im_end|>"]);
        assert_eq!(scan.push("Answer \n"), go("Answer"));
        assert_eq!(scan.push("<|im_end|>"), Scan::Stopped(String::new()));
    }

    #[test]
    fn releases_held_whitespace_once_text_follows() {
        let mut scan = scanner(&["STOP"]);
        assert_eq!(scan.push("one "), go("one"));
        assert_eq!(scan.push("two"), go(" two"));
    }

    #[test]
    fn finish_releases_held_text_untrimmed() {
        let mut scan = scanner(&["</s>"]);
        assert_eq!(scan.push("end \n</"), go("end"));
        assert_eq!(scan.finish(), " \n</");
    }

    #[test]
    fn handles_multi_byte_text() {
        let mut scan = scanner(&["éé"]);
        assert_eq!(scan.push("café"), go("caf"));
        assert_eq!(scan.push("ü"), go("éü"));
        assert_eq!(scan.push("😀é"), go("😀"));
        assert_eq!(scan.push("é ignored"), Scan::Stopped(String::new()));
    }

    #[test]
    fn stops_at_the_earliest_of_several_stop_tokens() {
        let mut scan = scanner(&["BB", "A"]);
        assert_eq!(scan.push("xBByA"), Scan::Stopped("x".to_owned()));
    }

    #[test]
    fn a_stop_token_inside_one_piece_drops_the_rest() {
        let mut scan = scanner(&["\n\nUser:"]);
        assert_eq!(
            scan.push("Sure.\n\nUser: more"),
            Scan::Stopped("Sure.".to_owned())
        );
    }

    #[test]
    fn ignores_empty_stop_tokens() {
        let mut scan = scanner(&[""]);
        assert_eq!(scan.push("text"), go("text"));
    }
}
