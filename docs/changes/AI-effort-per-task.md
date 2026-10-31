# AI-effort-per-task · Choose how hard Claude thinks, per task

## Progress log (PARITY_PLAN §7.1)
Settings → Models per task already let the user pick the provider and model
for each task (`ai.routes`). A Claude task now also has an effort: Default
(the model's own), Low, Medium, High, Extra-high or Max.

- `deino_llm::Target` carries `effort: Option<Effort>` (serialized only when
  set, so existing `ai.routes` read unchanged); `CompletionRequest.effort`
  comes from the route's target, and the Anthropic provider sends it as
  `output_config.effort`, beside a structured output's `format`. Other
  providers ignore it.
- UI: `RouteRow` shows an effort picker when the task's provider is
  Anthropic; changing the model keeps the effort, changing the provider drops it.

## Follow-ups
- Map effort to OpenAI's `reasoning_effort` for OpenAI-compatible providers.
- Claude Haiku 4.5 takes no effort; pinning one there makes its calls fail
  (the row's Test shows it).
