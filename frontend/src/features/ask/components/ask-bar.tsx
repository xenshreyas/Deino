/**
 * The ask bar: a question, or `/` for a recipe. Typing `/` lists the
 * runnable recipes (↑ ↓ to move, Enter or Tab to choose, Escape to close);
 * the chosen one shows as a tag the user can add words after, and Backspace
 * on an empty field removes it. `/slug words` typed out in full works too.
 * Enter sends, Shift+Enter starts a new line.
 */
import { type KeyboardEvent, useId, useState } from 'react';

import { isRunnable, type Recipe, slashMatches } from '@/features/recipes';
import { cn } from '@/ui/cn';
import { ArrowUp, X } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { slashQuery, typedRecipe } from '../answer-text';
import { askCopy } from '../copy';
import type { AskInput } from '../types';
import { SlashMenu } from './slash-menu';

/** Props of {@link AskBar}. */
export interface AskBarProps {
  recipes: readonly Recipe[];
  busy: boolean;
  onSend: (input: AskInput) => void;
}

/** What `text` and the chosen recipe ask, or `null` when nothing. */
function toInput(text: string, chosen: Recipe | null, recipes: readonly Recipe[]): AskInput | null {
  const typed = chosen === null ? typedRecipe(text) : null;
  const found = recipes.find((r) => r.slug === typed?.slug && isRunnable(r)) ?? null;
  const recipe = chosen ?? found;
  const question = found !== null && typed !== null ? typed.rest : text.trim();
  if (recipe === null && question === '') return null;
  const label = recipe === null ? null : { slug: recipe.slug, name: recipe.name };
  return { request: { question, recipe: recipe?.id ?? null }, label };
}

/** The `/` menu's highlight and keys: ↑ ↓ move, Enter or Tab choose, Escape closes. */
function useMenuKeys(
  matches: readonly Recipe[],
  pick: (recipe: Recipe) => void,
  setText: (text: string) => void,
) {
  const [active, setActive] = useState(0);
  const count = matches.length;
  /** Handles a menu key; false for any other key. */
  const onMenuKey = (event: KeyboardEvent<HTMLTextAreaElement>): boolean => {
    const chosen = matches[active] ?? matches[0];
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      setActive((active + (event.key === 'ArrowDown' ? 1 : count - 1)) % count);
    } else if (event.key === 'Enter' || event.key === 'Tab') {
      if (chosen !== undefined) pick(chosen);
      setActive(0);
    } else if (event.key === 'Escape') {
      setText('');
    } else {
      return false;
    }
    event.preventDefault();
    return true;
  };
  return { active, onMenuKey };
}

/** The bar's state and keyboard. */
function useAskBar({ recipes, busy, onSend }: AskBarProps) {
  const [text, setText] = useState('');
  const [recipe, setRecipe] = useState<Recipe | null>(null);
  const query = recipe === null ? slashQuery(text) : null;
  const matches = query === null ? [] : slashMatches(recipes, query);
  const pick = (chosen: Recipe | null) => {
    setRecipe(chosen);
    setText('');
  };
  const { active, onMenuKey } = useMenuKeys(matches, pick, setText);
  const input = toInput(text, recipe, recipes);
  const submit = () => {
    if (input === null || busy) return;
    onSend(input);
    pick(null);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (matches.length > 0 && onMenuKey(event)) return;
    if (event.key === 'Backspace' && text === '' && recipe !== null) setRecipe(null);
    if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      submit();
    }
  };
  const menuOpen = query !== null;
  return {
    text,
    setText,
    recipe,
    setRecipe,
    pick,
    active,
    matches,
    menuOpen,
    input,
    submit,
    onKeyDown,
  };
}

/** What the bar's field reads. */
type BarState = ReturnType<typeof useAskBar>;

/** The bar. */
export function AskBar(props: AskBarProps) {
  const bar = useAskBar(props);
  const menuId = useId();
  return (
    <div className="relative shrink-0 border-t border-border p-3">
      {bar.menuOpen ? (
        <SlashMenu id={menuId} recipes={bar.matches} active={bar.active} onPick={bar.pick} />
      ) : null}
      <div
        className={cn(
          'flex items-end gap-1.5 rounded-lg border border-control-border bg-control py-1.5 pr-1.5 pl-2.5',
          'focus-within:outline-2 focus-within:outline-focus-ring',
        )}
      >
        <Field bar={bar} menuId={menuId} />
        <IconButton
          icon={ArrowUp}
          label={askCopy.send}
          disabled={bar.input === null || props.busy}
          onClick={bar.submit}
          className="bg-fg text-on-fg hover:bg-fg hover:text-on-fg hover:opacity-90"
        />
      </div>
    </div>
  );
}

/** The recipe tag and the text field. */
function Field({ bar, menuId }: { bar: BarState; menuId: string }) {
  return (
    <div className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5 py-0.5">
      {bar.recipe === null ? null : (
        <RecipeTag
          recipe={bar.recipe}
          onRemove={() => {
            bar.setRecipe(null);
          }}
        />
      )}
      <textarea
        rows={2}
        value={bar.text}
        aria-label={askCopy.placeholder}
        aria-controls={bar.menuOpen ? menuId : undefined}
        placeholder={bar.recipe === null ? askCopy.placeholder : undefined}
        onChange={(event) => {
          bar.setText(event.target.value);
        }}
        onKeyDown={bar.onKeyDown}
        className="max-h-40 min-w-0 flex-1 resize-none bg-transparent text-sm text-fg outline-none placeholder:text-fg-subtle"
      />
    </div>
  );
}

/** The chosen recipe in the bar. */
function RecipeTag({ recipe, onRemove }: { recipe: Recipe; onRemove: () => void }) {
  return (
    <span className="inline-flex h-6 items-center gap-0.5 rounded-sm bg-bg-muted pr-0.5 pl-1.5 font-mono text-xs text-fg">
      /{recipe.slug}
      <button
        type="button"
        aria-label={askCopy.removeRecipe}
        onClick={onRemove}
        className="flex size-4 items-center justify-center rounded-sm text-fg-muted hover:text-fg"
      >
        <X size={12} strokeWidth={1.5} aria-hidden />
      </button>
    </span>
  );
}
