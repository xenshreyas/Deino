/**
 * The ask feature's types. Shapes shared with Rust are generated
 * (`src-tauri/src/bindings.rs`), never written by hand.
 */
import type { AskRequest, RecipeLabel } from '@/lib/ipc/generated/ask';

export type {
  AnswerCitation,
  AnswerPart,
  AskMessage,
  AskRequest,
  AskRole,
  AskStreamEvent,
  RecipeLabel,
} from '@/lib/ipc/generated/ask';

/** What the bar sends: the request and, for a recipe, how the thread labels it. */
export interface AskInput {
  request: AskRequest;
  label: RecipeLabel | null;
}

/** A question on its way: shown at once, its answer streaming in. */
export interface PendingAsk {
  question: string;
  recipe: RecipeLabel | null;
  /** The answer so far, `[s12]` markers and all. */
  streamed: string;
}
