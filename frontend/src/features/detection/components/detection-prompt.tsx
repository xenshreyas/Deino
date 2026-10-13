/**
 * The prompt window's content: the current detection, wired to Rust.
 *
 * Each detection remounts the card (by `key`), which restarts its entrance,
 * its countdown and its theme: the main window may have switched theme
 * since the last prompt, and only its first-paint cache reaches this window.
 */
import { applyTheme, parseThemePreference, THEME_STORAGE_KEY } from '@/features/appearance';
import { useMountEffect } from '@/hooks/useMountEffect';

import { millisecondsLeft, secondsLeft, useNow } from '../countdown';
import { type PromptAnswer, useAnswerPrompt, usePromptDetection } from '../queries';
import type { Detection } from '../types';
import { MeetingPrompt } from './meeting-prompt';

/** How long a prompt stays up unanswered; Rust's `PROMPT_TIMEOUT`. */
export const PROMPT_OPEN_MS = 30_000;

/** When an unanswered prompt closes on its own. */
function promptClosesAt(detection: Detection): string {
  return new Date(Date.parse(detection.detectedAt) + PROMPT_OPEN_MS).toISOString();
}

/** Applies the theme the main window last saved. */
function syncTheme(): void {
  try {
    applyTheme(parseThemePreference(window.localStorage.getItem(THEME_STORAGE_KEY)));
  } catch (error: unknown) {
    console.error('[detection] Could not read the saved theme', error);
  }
}

/** One detection's card, with its countdown and answers. */
function PromptCard({ detection }: { detection: Detection }) {
  const answer = useAnswerPrompt();
  const now = useNow();
  useMountEffect(syncTheme);
  const deadline = detection.autoStartAt;
  const closesAt = deadline ?? promptClosesAt(detection);
  const send = (choice: PromptAnswer) => () => {
    answer.mutate({ id: detection.id, answer: choice });
  };
  return (
    <MeetingPrompt
      detection={detection}
      secondsLeft={deadline === null ? null : secondsLeft(deadline, now)}
      millisecondsLeft={millisecondsLeft(closesAt, now)}
      busy={answer.isPending}
      failed={answer.isError}
      onTakeNotes={send('takeNotes')}
      onDismiss={send('dismiss')}
      onAlwaysIgnore={send('ignoreApp')}
    />
  );
}

/** The prompt for the detection on screen; nothing between prompts. */
export function DetectionPrompt() {
  const detection = usePromptDetection();
  return detection === null ? null : <PromptCard key={detection.id} detection={detection} />;
}
