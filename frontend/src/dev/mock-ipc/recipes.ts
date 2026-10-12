/**
 * Mock of `@/lib/ipc/generated/recipes`: the built-in library (names and
 * slugs as `deino_core::recipes::LIBRARY` has them, shorter prompts) and the
 * user's own recipes, in memory, with Rust's rules: unique slugs from names,
 * read-only built-ins, Remix, and the share string.
 *
 * It imports no feature code, so the feature reaching it through its
 * generated bindings forms no cycle.
 */
import type * as Generated from '@/lib/ipc/generated/recipes';

import { failure, ok } from './command-result';
import { latency } from './latency';

type Recipe = Generated.Recipe;

const AT = '2026-10-12T09:00:00Z';

/** `[slug, name, description, multi?]` of each built-in, in library order. */
const LIBRARY: readonly (readonly [string, string, string, boolean?])[] = [
  ['coach-me', 'Coach me', 'Feedback on how you showed up, tied to moments in the meeting.'],
  [
    'prep-me-for-this-person',
    'Prep me for this person',
    'What to remember before you next talk to the people you met.',
  ],
  ['write-a-brief', 'Write a brief', 'A short brief for someone who missed the meeting.'],
  [
    'write-follow-up-email',
    'Write follow-up email',
    'A follow-up email to the attendees, in your voice.',
  ],
  [
    'summarize-for-my-manager',
    'Summarize for my manager',
    'Outcomes, risks and asks, the way a manager reads them.',
  ],
  ['extract-decisions', 'Extract decisions', 'Every decision, who made it and why.'],
  [
    'list-open-questions',
    'List open questions',
    'What is still unanswered, and who could answer it.',
  ],
  [
    'customer-pain-points',
    'Customer pain points',
    'The problems the customer described, in their words.',
  ],
  ['feature-requests', 'Feature requests', 'What was asked for, and why it matters to them.'],
  [
    'competitor-mentions',
    'Competitor mentions',
    'Every competitor named, and what was said about them.',
  ],
  [
    'objections-and-responses',
    'Objections & responses',
    'Each objection raised and how it was answered.',
  ],
  ['hiring-scorecard', 'Hiring scorecard', 'A candidate scorecard with evidence for each rating.'],
  [
    'prd-draft',
    'PRD draft from discussion',
    'A product requirements draft from what was discussed.',
  ],
  ['weekly-recap', 'Weekly recap', 'A recap of your week across meetings.', true],
  [
    'how-have-i-changed',
    'How have I changed',
    'How your focus and habits shifted over time.',
    true,
  ],
  ['how-i-like-to-work', 'How I like to work', 'Your working style, as this meeting shows it.'],
  ['prep-my-day', 'Prep my day', "Get ready for today's meetings.", true],
  [
    'mochary-1-1-review',
    'Mochary-style 1:1 review',
    'Review a 1:1 the Mochary way: issues, owners, commitments.',
  ],
  ['risks-and-blockers', 'Risks & blockers', 'Risks and blockers, with owners and mitigations.'],
  ['what-did-i-commit-to', 'What did I commit to?', 'Everything you promised, with dates.'],
];

let recipes: Recipe[] = LIBRARY.map(([slug, name, description, multi], index) => ({
  id: `00000000-0000-7000-8000-${String(index).padStart(12, '0')}` as Generated.RecipeId,
  slug,
  name,
  description,
  prompt: `${description} Use only what this meeting says.`,
  kind: multi === true ? 'multi' : 'single',
  builtIn: true,
  createdAt: AT,
  updatedAt: AT,
}));

function slugify(name: string): string {
  const slug = name
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean)
    .join('-')
    .slice(0, 48);
  return slug.replace(/-+$/, '') || 'recipe';
}

function unique(base: string, except?: string): string {
  const taken = new Set(recipes.filter((r) => r.id !== except).map((r) => r.slug));
  if (!taken.has(base)) return base;
  let n = 2;
  while (taken.has(`${base}-${String(n)}`)) n += 1;
  return `${base}-${String(n)}`;
}

function create(draft: Generated.RecipeDraft): Recipe {
  const recipe: Recipe = {
    id: crypto.randomUUID() as Generated.RecipeId,
    slug: unique(slugify(draft.name)),
    name: draft.name.trim(),
    description: draft.description.trim(),
    prompt: draft.prompt.trim(),
    kind: 'single',
    builtIn: false,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
  recipes = [...recipes, recipe];
  return recipe;
}

function find(id: Generated.RecipeId): Recipe | undefined {
  return recipes.find((recipe) => recipe.id === id);
}

const readOnly = failure<never>({
  code: 'permission_denied',
  message: 'Built-in recipes cannot be changed; remix it to make your own copy.',
});
const missing = failure<never>({ code: 'not_found', message: 'That recipe no longer exists.' });

function shared(recipe: Recipe) {
  return {
    name: recipe.name,
    description: recipe.description,
    prompt: recipe.prompt,
    kind: recipe.kind,
  };
}

function parse(text: string): Generated.RecipeDraft[] {
  const trimmed = text.trim();
  const json = trimmed.startsWith('deino://recipe?data=')
    ? atob(trimmed.slice('deino://recipe?data='.length).replaceAll('-', '+').replaceAll('_', '/'))
    : trimmed;
  const value: unknown = JSON.parse(json);
  const list = (value as { recipes?: Generated.RecipeDraft[] }).recipes ?? [
    value as Generated.RecipeDraft,
  ];
  return list.map((r) => ({ name: r.name, description: r.description, prompt: r.prompt }));
}

export const commands = {
  async list() {
    await latency();
    return ok(recipes);
  },
  async create(draft) {
    await latency();
    return ok(create(draft));
  },
  async update(id, draft) {
    await latency();
    const current = find(id);
    if (current === undefined) return missing;
    if (current.builtIn) return readOnly;
    const updated = {
      ...current,
      ...draft,
      slug: unique(slugify(draft.name), id),
      updatedAt: new Date().toISOString(),
    };
    recipes = recipes.map((recipe) => (recipe.id === id ? updated : recipe));
    return ok(updated);
  },
  async remove(id) {
    await latency();
    if (find(id)?.builtIn === true) return readOnly;
    recipes = recipes.filter((recipe) => recipe.id !== id);
    return ok(null);
  },
  async remix(id) {
    await latency();
    const original = find(id);
    if (original === undefined) return missing;
    return ok(
      create({
        name: `${original.name} (remix)`,
        description: original.description,
        prompt: original.prompt,
      }),
    );
  },
  async exportJson(ids) {
    await latency();
    const chosen =
      ids.length === 0
        ? recipes.filter((r) => !r.builtIn)
        : recipes.filter((r) => ids.includes(r.id));
    return ok(
      JSON.stringify({ format: 'deino.recipes', version: 1, recipes: chosen.map(shared) }, null, 2),
    );
  },
  async importShared(text) {
    await latency();
    try {
      return ok(parse(text).map(create));
    } catch {
      return failure({
        code: 'invalid',
        message: 'This is not a Deino recipe: expected a recipes file or a recipe link.',
      });
    }
  },
  async shareLink(id) {
    await latency();
    const recipe = find(id);
    if (recipe === undefined) return missing;
    const data = btoa(JSON.stringify(shared(recipe)))
      .replaceAll('+', '-')
      .replaceAll('/', '_')
      .replace(/=+$/, '');
    return ok(`deino://recipe?data=${data}`);
  },
} satisfies typeof Generated.commands;
