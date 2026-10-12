import { describe, expect, it } from 'vitest';

import { filterRecipes, isRunnable, slashMatches } from './filter';
import type { Recipe, RecipeId } from './types';

function recipe(slug: string, overrides: Partial<Recipe> = {}): Recipe {
  return {
    id: `id-${slug}` as RecipeId,
    slug,
    name: slug.replaceAll('-', ' '),
    description: '',
    prompt: `Do ${slug}`,
    kind: 'single',
    builtIn: true,
    createdAt: '2026-10-12T09:00:00Z',
    updatedAt: '2026-10-12T09:00:00Z',
    ...overrides,
  };
}

const ALL = [
  recipe('coach-me'),
  recipe('write-a-brief', { prompt: 'Summarize the pricing talk' }),
  recipe('weekly-recap', { kind: 'multi' }),
  recipe('my-brief', { builtIn: false }),
];

describe('filterRecipes', () => {
  it('splits built-ins from the user’s own', () => {
    expect(filterRecipes(ALL, 'mine', '').map((r) => r.slug)).toEqual(['my-brief']);
    expect(filterRecipes(ALL, 'built-in', '')).toHaveLength(3);
  });

  it('searches names, slugs and prompts', () => {
    expect(filterRecipes(ALL, 'built-in', 'PRICING').map((r) => r.slug)).toEqual(['write-a-brief']);
  });
});

describe('slashMatches', () => {
  it('lists runnable recipes, prefix matches first', () => {
    expect(slashMatches(ALL, 'brief').map((r) => r.slug)).toEqual(['write-a-brief', 'my-brief']);
    expect(slashMatches(ALL, 'my').map((r) => r.slug)).toEqual(['my-brief']);
  });

  it('never offers a multi-meeting recipe', () => {
    expect(slashMatches(ALL, '').some((r) => !isRunnable(r))).toBe(false);
    expect(slashMatches(ALL, 'weekly')).toEqual([]);
  });
});
