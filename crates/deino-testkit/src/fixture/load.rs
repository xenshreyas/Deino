//! Reading `fixtures/` into typed meetings, validating as it goes.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use deino_domain::MeetingId;

use super::model::{FixtureAudio, FixtureMeeting};
use super::validate::{Problem, validate};
use super::vocab::{Channel, MeetingKind};

/// Why the fixtures could not be loaded. Messages name files and IDs only.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// A file or directory could not be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file or directory.
        path: PathBuf,
        /// The I/O failure.
        source: io::Error,
    },
    /// A file is not valid JSON or does not match the schema.
    #[error("{} does not match the fixture schema", path.display())]
    Parse {
        /// The fixture file.
        path: PathBuf,
        /// What serde rejected, with its line and column.
        source: serde_json::Error,
    },
    /// A meeting breaks a cross-field rule.
    #[error("fixture {meeting} is invalid")]
    Invalid {
        /// The meeting.
        meeting: MeetingId,
        /// The rule it breaks.
        source: Problem,
    },
    /// Two files use the same meeting ID.
    #[error("fixture id {0} is used by two files")]
    DuplicateId(MeetingId),
    /// A meeting names an audio file that does not exist.
    #[error("fixture {meeting} names missing audio {}", path.display())]
    MissingAudio {
        /// The meeting.
        meeting: MeetingId,
        /// The missing file.
        path: PathBuf,
    },
}

/// The fixture meetings in one `fixtures/` directory, in file-name order.
#[derive(Clone, Debug)]
pub struct Fixtures {
    root: PathBuf,
    meetings: Vec<FixtureMeeting>,
}

impl Fixtures {
    /// The repository's own `fixtures/` directory.
    pub fn repository_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    /// Loads and validates the repository's fixtures.
    pub fn load() -> Result<Self, FixtureError> {
        Self::load_from(Self::repository_root())
    }

    /// Loads and validates `root/meetings/*.json`, checking that every audio
    /// file a meeting names exists under `root`.
    pub fn load_from(root: impl Into<PathBuf>) -> Result<Self, FixtureError> {
        let root = root.into();
        let mut meetings = Vec::new();
        let mut ids = HashSet::new();
        for path in json_files(&root.join("meetings"))? {
            let meeting = parse(&path)?;
            if !ids.insert(meeting.id.clone()) {
                return Err(FixtureError::DuplicateId(meeting.id));
            }
            meetings.push(meeting);
        }
        let fixtures = Self { root, meetings };
        fixtures.check_audio_files()?;
        Ok(fixtures)
    }

    /// Every meeting, in file-name order.
    pub fn meetings(&self) -> &[FixtureMeeting] {
        &self.meetings
    }

    /// The meeting of `kind`, if a fixture covers it.
    pub fn meeting(&self, kind: MeetingKind) -> Option<&FixtureMeeting> {
        self.meetings.iter().find(|meeting| meeting.kind == kind)
    }

    /// The file of one track of a meeting's audio; `None` for
    /// [`Channel::Mixed`], since fixtures keep the tracks apart.
    pub fn audio_path(&self, audio: &FixtureAudio, channel: Channel) -> Option<PathBuf> {
        let relative = match channel {
            Channel::Mic => &audio.mic,
            Channel::System => &audio.system,
            Channel::Mixed => return None,
        };
        Some(self.root.join(relative))
    }

    fn check_audio_files(&self) -> Result<(), FixtureError> {
        for meeting in &self.meetings {
            let Some(audio) = &meeting.audio else {
                continue;
            };
            for path in [&audio.mic, &audio.system].map(|track| self.root.join(track)) {
                if !path.is_file() {
                    let meeting = meeting.id.clone();
                    return Err(FixtureError::MissingAudio { meeting, path });
                }
            }
        }
        Ok(())
    }
}

/// The `.json` files directly inside `directory`, sorted by name.
fn json_files(directory: &Path) -> Result<Vec<PathBuf>, FixtureError> {
    let read_error = |source| FixtureError::Read {
        path: directory.to_path_buf(),
        source,
    };
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(read_error)? {
        let path = entry.map_err(read_error)?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Reads, deserializes and validates one fixture file.
fn parse(path: &Path) -> Result<FixtureMeeting, FixtureError> {
    let text = std::fs::read_to_string(path).map_err(|source| FixtureError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let meeting: FixtureMeeting =
        serde_json::from_str(&text).map_err(|source| FixtureError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    validate(&meeting).map_err(|source| FixtureError::Invalid {
        meeting: meeting.id.clone(),
        source,
    })?;
    Ok(meeting)
}
