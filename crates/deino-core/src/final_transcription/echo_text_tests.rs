use super::*;

/// A line whose words start at `start`, 300 ms apart, each 250 ms long.
fn timed(start: u32, text: &str) -> HeardSpan {
    let words: Vec<WordTiming> = (0_u32..)
        .zip(text.split_whitespace())
        .map(|(index, word)| WordTiming {
            text: word.to_owned(),
            start: Millis(start + index * 300),
            end: Millis(start + index * 300 + 250),
        })
        .collect();
    HeardSpan {
        start: Millis(start),
        end: words.last().map_or(Millis(start), |word| word.end),
        text: text.to_owned(),
        words,
    }
}

/// The same line without word times, as the Whisper and Parakeet TDT
/// engines give it.
fn untimed(start: u32, text: &str) -> HeardSpan {
    HeardSpan {
        words: Vec::new(),
        ..timed(start, text)
    }
}

/// The Me lines left after the rule, as text.
fn me_after(heard: Vec<(Channel, HeardSpan)>) -> Vec<String> {
    without_echo(heard)
        .into_iter()
        .filter(|(channel, _)| *channel == Channel::Mic)
        .map(|(_, line)| line.text)
        .collect()
}

const THEM: &str = "the quarterly numbers look good for the northeast region";

#[test]
fn a_fully_duplicated_segment_is_dropped() {
    let timed_echo = vec![
        (Channel::System, timed(10_000, THEM)),
        (Channel::Mic, timed(10_040, THEM)),
    ];
    let untimed_echo = vec![
        (Channel::System, untimed(10_000, THEM)),
        (
            Channel::Mic,
            untimed(
                10_300,
                "The quarterly number looks good for northeast regions.",
            ),
        ),
    ];

    assert!(me_after(timed_echo).is_empty());
    assert!(me_after(untimed_echo).is_empty());
}

#[test]
fn a_mixed_segment_keeps_only_the_users_own_words() {
    let them = timed(10_000, THEM);
    let mut mine = timed(9_100, "sounds good to me");
    let echo = timed(
        10_080,
        "the quarterly numbers look god for the northeast region",
    );
    mine.words.extend(echo.words);
    mine.text = format!("{} {}", mine.text, echo.text);
    mine.end = echo.end;

    let after = without_echo(vec![(Channel::System, them), (Channel::Mic, mine)]);

    let (_, kept) = after.iter().find(|(c, _)| *c == Channel::Mic).unwrap();
    assert_eq!(kept.text, "sounds good to me");
    assert_eq!((kept.start, kept.end), (Millis(9_100), Millis(10_250)));
    assert_eq!(kept.words.len(), 4);
}

#[test]
fn a_real_reply_that_repeats_a_word_or_two_is_kept() {
    let heard = vec![
        (Channel::System, timed(10_000, "can we ship it on friday")),
        (Channel::Mic, timed(11_900, "yes friday works for me")),
        (Channel::System, untimed(20_000, "can we ship it on friday")),
        (Channel::Mic, untimed(21_900, "yes friday is fine")),
    ];

    assert_eq!(
        me_after(heard),
        ["yes friday works for me", "yes friday is fine"]
    );
}

#[test]
fn the_same_words_at_another_time_are_kept() {
    let heard = vec![
        (Channel::System, timed(10_000, THEM)),
        (Channel::Mic, timed(40_000, THEM)),
        (Channel::Mic, untimed(60_000, THEM)),
    ];

    assert_eq!(me_after(heard), [THEM, THEM]);
}

#[test]
fn without_them_or_off_the_microphone_nothing_changes() {
    let alone = vec![(Channel::Mic, timed(0, THEM))];
    let mixed = vec![
        (Channel::Mixed, timed(0, THEM)),
        (Channel::System, timed(0, THEM)),
    ];

    assert_eq!(without_echo(alone.clone()), alone);
    assert_eq!(without_echo(mixed.clone()), mixed);
}

#[test]
fn cloud_lines_are_cleaned_too() {
    let line = |span| CloudSpan { span, voice: None };
    let heard = vec![
        (Channel::System, line(timed(10_000, THEM))),
        (Channel::Mic, line(timed(10_000, THEM))),
    ];

    let after = without_echo(heard);

    assert_eq!(after.len(), 1);
    assert_eq!(after[0].0, Channel::System);
}
