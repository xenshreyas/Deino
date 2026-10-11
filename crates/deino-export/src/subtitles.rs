//! The transcript as subtitles: SubRip (SRT) and WebVTT, one cue per
//! utterance, each naming its speaker, so the recording plays with captions
//! in `QuickTime`, VLC or a video editor.

use deino_domain::Millis;

use crate::labels::subtitle_time;
use crate::{MeetingExport, Utterance};

/// The transcript as SRT: numbered cues, `Speaker: text`.
pub fn srt(export: &MeetingExport) -> String {
    export
        .transcript
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let (start, end) = span(line);
            format!(
                "{}\n{} --> {}\n{}: {}\n",
                index + 1,
                subtitle_time(start, ','),
                subtitle_time(end, ','),
                one_line(&line.speaker),
                one_line(&line.text)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The transcript as WebVTT, with `<v Speaker>` voice spans.
pub fn vtt(export: &MeetingExport) -> String {
    let cues: Vec<String> = export
        .transcript
        .iter()
        .map(|line| {
            let (start, end) = span(line);
            format!(
                "{} --> {}\n<v {}>{}\n",
                subtitle_time(start, '.'),
                subtitle_time(end, '.'),
                escape_vtt(&one_line(&line.speaker)),
                escape_vtt(&one_line(&line.text))
            )
        })
        .collect();
    let mut file = String::from("WEBVTT\n");
    for cue in cues {
        file.push('\n');
        file.push_str(&cue);
    }
    file
}

/// A cue's times; an end before its start is clamped so players accept it.
fn span(line: &Utterance) -> (Millis, Millis) {
    (line.start, line.end.max(line.start))
}

/// A blank line ends a cue, so text keeps to one line.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn escape_vtt(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::fixture_export;

    #[test]
    fn the_standup_and_one_on_one_render_as_srt_and_vtt() {
        for name in ["standup", "one-on-one"] {
            let export = fixture_export(name);
            insta::assert_snapshot!(format!("srt_{name}"), srt(&export));
            insta::assert_snapshot!(format!("vtt_{name}"), vtt(&export));
        }
    }

    #[test]
    fn cue_text_cannot_break_the_file() {
        let mut export = fixture_export("standup");
        export.transcript.truncate(1);
        export.transcript[0].text = "a\n\nb --> <c> & d".into();
        export.transcript[0].end = Millis(0);

        assert_eq!(
            srt(&export),
            "1\n00:00:00,696 --> 00:00:00,696\nShreyas: a b --> <c> & d\n"
        );
        assert_eq!(
            vtt(&export),
            "WEBVTT\n\n00:00:00.696 --> 00:00:00.696\n<v Shreyas>a b --&gt; &lt;c&gt; &amp; d\n"
        );
    }
}
