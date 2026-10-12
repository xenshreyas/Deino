//! [`GatewayDescriber`]: `vision.describe` over the LLM gateway (FND-09).
//!
//! A picture leaves the Mac only when the user turned on "Describe
//! snapshots with Claude" and the route is served by Claude: another
//! provider (a local model, an OpenAI-compatible one) is never sent the
//! picture, so describing is simply off for it. Each call sends one PNG and
//! a fixed request, nothing else about the meeting.

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use deino_llm::prompts::{PromptText, VisionDescribeSystem, VisionDescribeUser};
use deino_llm::{Gateway, ImagePart, Message, Part, Prompt, ProviderId, Role, Route};

use super::{DescribeError, SnapshotDescriber};
use crate::settings::SettingsService;

/// The longest description, in tokens.
const DESCRIBE_MAX_TOKENS: u32 = 400;
/// The snapshots' media type.
const PNG: &str = "image/png";

/// Settings → Screen capture → "Describe snapshots with Claude".
#[async_trait]
pub trait DescribeSetting: Send + Sync {
    /// Whether the user turned describing on.
    async fn describe_snapshots(&self) -> bool;
}

#[async_trait]
impl DescribeSetting for SettingsService {
    async fn describe_snapshots(&self) -> bool {
        self.get().await.screen.describe_snapshots
    }
}

/// `vision.describe` through the gateway (see the module docs).
pub struct GatewayDescriber {
    gateway: Arc<Gateway>,
    setting: Arc<dyn DescribeSetting>,
}

impl GatewayDescriber {
    /// A describer over the gateway and the setting.
    pub fn new(gateway: Arc<Gateway>, setting: Arc<dyn DescribeSetting>) -> Self {
        Self { gateway, setting }
    }
}

/// The prompt that sends `png` to be described.
pub fn describe_prompt(png: &[u8]) -> Prompt {
    let image = Part::Image(ImagePart {
        media_type: PNG.to_owned(),
        base64: STANDARD.encode(png),
    });
    Prompt {
        system: Some(VisionDescribeSystem.text()),
        messages: vec![Message {
            role: Role::User,
            parts: vec![image, Part::Text(VisionDescribeUser.text())],
        }],
        max_tokens: Some(DESCRIBE_MAX_TOKENS),
        ..Prompt::default()
    }
}

#[async_trait]
impl SnapshotDescriber for GatewayDescriber {
    async fn enabled(&self) -> bool {
        if !self.setting.describe_snapshots().await {
            return false;
        }
        let target = self
            .gateway
            .target(Route::VisionDescribe, &Prompt::default())
            .await;
        matches!(target, Ok(target) if target.provider == ProviderId::Anthropic)
    }

    async fn describe(&self, png: Vec<u8>) -> Result<String, DescribeError> {
        let answer = self
            .gateway
            .complete(Route::VisionDescribe, describe_prompt(&png))
            .await
            .map_err(|error| DescribeError(error.to_string()))?;
        Ok(answer.text.trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_goes_first_with_a_fixed_request() {
        let prompt = describe_prompt(&[0x89, b'P', b'N', b'G']);

        let [message] = prompt.messages.as_slice() else {
            panic!("one message");
        };
        assert_eq!(message.role, Role::User);
        assert_eq!(
            message.parts.first(),
            Some(&Part::Image(ImagePart {
                media_type: "image/png".to_owned(),
                base64: "iVBORw==".to_owned(),
            }))
        );
        assert_eq!(message.text(), "Describe this shared screen.");
        assert!(
            prompt
                .system
                .is_some_and(|system| system.contains("only what is visible"))
        );
    }
}
