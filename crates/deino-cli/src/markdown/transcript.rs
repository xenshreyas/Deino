//! A transcript page as Markdown: one `[m:ss] Speaker: text` line each.

use deino_core::query::TranscriptPage;
use deino_core::query::link::{clock_time, moment_link};
use deino_domain::Millis;

use super::line;
use crate::request::Markdown;

impl Markdown for TranscriptPage {
    fn markdown(&self) -> String {
        let mut out = String::new();
        line(&mut out, format!("# {} — transcript\n", self.title));
        if self.lines.is_empty() {
            line(&mut out, "No lines in this window.");
            return out;
        }
        let first = self.lines.first().map_or(0, |line| line.start_ms);
        line(
            &mut out,
            format!(
                "`{}` · {} of {} lines · times are from the start; link a moment as {}\n",
                self.meeting_id,
                self.lines.len(),
                self.total_lines,
                moment_link(&self.meeting_id, Millis(first)),
            ),
        );
        for spoken in &self.lines {
            let time = clock_time(Millis(spoken.start_ms));
            line(
                &mut out,
                format!("[{time}] {}: {}", spoken.speaker, spoken.text),
            );
        }
        if let Some(next) = self.next_from_ms {
            line(
                &mut out,
                format!("\nMore follows: continue with from_ms={next} (--from-ms {next})."),
            );
        }
        out
    }
}
