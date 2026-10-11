//! Fakes for other crates' tests (behind `test-support`).

use std::sync::Arc;

use deino_stt::clock::SystemMonotonic;
use deino_stt::transcriber::FakeTranscriber;

use crate::engine::{
    EngineError, LaneEngines, LaneRequest, LaneTranscriber, StreamingLane, StreamingParts,
};

/// Each lane streams through `deino-stt`'s scripted fake engine: every loud
/// 400 ms block of the lane's audio becomes a word named after the block.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScriptedEngines;

impl LaneEngines for ScriptedEngines {
    fn open(&self, request: LaneRequest) -> Result<Box<dyn LaneTranscriber>, EngineError> {
        let lane = StreamingLane::start(StreamingParts {
            lane: request.lane,
            transcriber: Arc::new(FakeTranscriber::new()),
            sink: request.sink,
            start: request.start,
            clock: Arc::new(SystemMonotonic::new()),
            keep_alive: Vec::new(),
        })?;
        Ok(Box::new(lane))
    }
}
