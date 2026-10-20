use super::*;

fn name(text: &str) -> DisplayName {
    DisplayName::parse(text).unwrap()
}

fn frame(width: u32, height: u32) -> Frame {
    Frame {
        x: 0,
        y: 0,
        width,
        height,
    }
}

/// A real-shaped tile: an `AXTabGroup` with an image and a name button.
fn tile_node(description: &str, frame: Option<Frame>) -> AxNode {
    let name = description.split(',').next().unwrap_or_default();
    AxNode {
        role: TILE_ROLE.into(),
        description: Some(description.into()),
        frame,
        children: vec![
            AxNode {
                role: "AXImage".into(),
                ..AxNode::default()
            },
            AxNode {
                role: BUTTON_ROLE.into(),
                description: Some(name.into()),
                ..AxNode::default()
            },
        ],
        ..AxNode::default()
    }
}

fn call(tiles: Vec<AxNode>) -> AxNode {
    AxNode {
        role: "AXWindow".into(),
        title: Some("Zoom Meeting".into()),
        children: tiles,
        ..AxNode::default()
    }
}

/// The main window with the account button for `own`.
fn workplace(own: &str) -> AxNode {
    AxNode {
        role: "AXWindow".into(),
        title: Some("Zoom Workplace".into()),
        children: vec![AxNode {
            role: BUTTON_ROLE.into(),
            description: Some(format!("Zoom, {own}, In a Zoom Meeting, Pro account")),
            ..AxNode::default()
        }],
        ..AxNode::default()
    }
}

fn read(window: &AxNode, context: &[AxNode]) -> Reading {
    let own = own_name(window, context);
    reading(&tiles(window), own.as_ref())
}

const ALEX_MUTED: &str = "Alex Chen, Computer audio muted, Video off";
const BEA_UNMUTED: &str = "Bea Ruiz, Computer audio unmuted";
const BEA_MUTED: &str = "Bea Ruiz, Computer audio muted";
const CLEO_UNMUTED: &str = "Cleo Martin, Computer audio unmuted, Video off";

#[test]
fn tiles_parse_name_audio_video_and_frame() {
    let window = call(vec![
        tile_node(ALEX_MUTED, None),
        tile_node(BEA_UNMUTED, Some(frame(1_261, 709))),
    ]);

    let tiles = tiles(&window);

    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0].label.name, name("Alex Chen"));
    assert!(!tiles[0].unmuted && !tiles[0].video);
    assert!(tiles[1].unmuted && tiles[1].video);
    assert_eq!(tiles[1].frame, Some(frame(1_261, 709)));
}

#[test]
fn the_roster_leaves_out_the_user_named_by_the_account_button() {
    let window = call(vec![
        tile_node(ALEX_MUTED, None),
        tile_node(BEA_MUTED, None),
        tile_node(CLEO_UNMUTED, None),
    ]);

    let reading = read(&window, &[workplace("Alex Chen")]);

    assert_eq!(
        reading.participants,
        [name("Bea Ruiz"), name("Cleo Martin")]
    );
    assert_eq!(own_name(&window, &[]), None);
    let unknown = read(&window, &[]);
    assert_eq!(unknown.participants.len(), 3);
}

#[test]
fn a_participants_row_marked_me_names_the_user_without_the_main_window() {
    let mut window = call(vec![
        tile_node(ALEX_MUTED, None),
        tile_node(BEA_MUTED, None),
    ]);
    window.children.push(AxNode {
        role: TEXT_ROLE.into(),
        value: Some("Alex Chen (Host, me)".into()),
        ..AxNode::default()
    });

    assert_eq!(own_name(&window, &[]), Some(name("Alex Chen")));
    assert_eq!(read(&window, &[]).participants, [name("Bea Ruiz")]);
}

#[test]
fn speaker_views_large_tile_is_the_speaker_even_when_muted_others_are_not() {
    let window = call(vec![
        tile_node(BEA_MUTED, Some(frame(1_261, 709))),
        tile_node(ALEX_MUTED, Some(frame(92, 52))),
        tile_node(CLEO_UNMUTED, Some(frame(92, 52))),
    ]);

    let reading = read(&window, &[workplace("Alex Chen")]);

    assert_eq!(reading.active, Active::Speaking(name("Bea Ruiz")));
}

#[test]
fn the_users_large_tile_is_their_own_voice() {
    let window = call(vec![
        tile_node(ALEX_MUTED, Some(frame(1_261, 709))),
        tile_node(BEA_UNMUTED, Some(frame(92, 52))),
    ]);

    assert_eq!(read(&window, &[workplace("Alex Chen")]).active, Active::Me);
}

#[test]
fn without_speaker_view_the_one_unmuted_remote_speaks() {
    // Gallery View: tiles of one size (or no frames at all).
    let gallery = call(vec![
        tile_node(ALEX_MUTED, Some(frame(600, 340))),
        tile_node(BEA_UNMUTED, Some(frame(600, 340))),
    ]);
    let unframed = call(vec![
        tile_node(BEA_UNMUTED, None),
        tile_node(ALEX_MUTED, None),
    ]);

    let own = [workplace("Alex Chen")];
    assert_eq!(
        read(&gallery, &own).active,
        Active::Speaking(name("Bea Ruiz"))
    );
    assert_eq!(
        read(&unframed, &own).active,
        Active::Speaking(name("Bea Ruiz"))
    );
}

#[test]
fn nobody_or_several_unmuted_name_nobody() {
    let own = [workplace("Alex Chen")];
    let silent = call(vec![
        tile_node(ALEX_MUTED, None),
        tile_node(BEA_MUTED, None),
    ]);
    let several = call(vec![
        tile_node(BEA_UNMUTED, None),
        tile_node(CLEO_UNMUTED, None),
        tile_node("Alex Chen, Computer audio unmuted", None),
    ]);
    // A large tile only four times the others is not Speaker View.
    let close = call(vec![
        tile_node(BEA_MUTED, Some(frame(400, 100))),
        tile_node(CLEO_UNMUTED, Some(frame(100, 100))),
        tile_node(ALEX_MUTED, Some(frame(100, 100))),
    ]);

    assert_eq!(read(&silent, &own).active, Active::Nobody);
    assert_eq!(read(&several, &own).active, Active::Several);
    assert!(read(&several, &own).active.remote().is_none());
    assert_eq!(
        read(&close, &own).active,
        Active::Speaking(name("Cleo Martin"))
    );
}

#[test]
fn elements_without_an_audio_state_are_not_tiles() {
    let window = call(vec![AxNode {
        role: TILE_ROLE.into(),
        description: Some("Settings, General".into()),
        ..AxNode::default()
    }]);

    assert!(tiles(&window).is_empty());
}
