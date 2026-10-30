use super::*;

/// Log-probabilities where frame `t` is sure of `classes[t]`.
fn sure_of(path: &[usize]) -> Vec<f32> {
    let classes = 32;
    let mut emission = vec![-20.0; path.len() * classes];
    for (t, &class) in path.iter().enumerate() {
        emission[t * classes + class] = 0.0;
    }
    emission
}

#[test]
fn letters_become_tokens_with_separators_between_words() {
    let spelled = tokens(&["It's", "42", "ok."]);

    // I=10 T=6 '=27 S=12, | =4, O=8 K=26; "42" has no letters.
    assert_eq!(spelled.ids, vec![10, 6, 27, 12, 4, 8, 26]);
    assert_eq!(
        spelled.owner,
        vec![Some(0), Some(0), Some(0), Some(0), None, Some(2), Some(2)]
    );
    assert_eq!(tokens(&["...", "42"]), Tokens::default());
}

#[test]
fn each_token_lands_on_the_frame_that_emits_it() {
    let ids = tokens(&["no", "go"]).ids; // N O | G O
    let path = [BLANK, 9, BLANK, 8, 4, BLANK, 21, 8, BLANK];

    let at = token_frames(&sure_of(&path), 32, &ids);

    assert_eq!(at, vec![1, 3, 4, 6, 7]);
}

#[test]
fn words_span_their_first_to_last_letter_frame() {
    let owner = [Some(0), Some(0), None, Some(2), Some(2)];

    let spans = word_frames(&[1, 3, 4, 6, 7], &owner, 3);

    assert_eq!(spans, vec![Some((1, 3)), None, Some((6, 7))]);
}

#[test]
fn unplaced_words_sit_between_their_neighbours_or_the_line_edges() {
    let mut times = vec![
        None,
        Some((100.0, 200.0)),
        None,
        None,
        Some((400.0, 500.0)),
        None,
    ];

    fill_gaps(&mut times, (50.0, 600.0));

    assert_eq!(
        times,
        vec![
            Some((50.0, 100.0)),
            Some((100.0, 200.0)),
            Some((200.0, 400.0)),
            Some((200.0, 400.0)),
            Some((400.0, 500.0)),
            Some((500.0, 600.0)),
        ]
    );
}

#[test]
fn a_gap_never_ends_before_it_starts() {
    let mut times = vec![Some((300.0, 350.0)), None, Some((320.0, 400.0))];

    fill_gaps(&mut times, (0.0, 500.0));

    assert_eq!(times[1], Some((350.0, 350.0)));
}
