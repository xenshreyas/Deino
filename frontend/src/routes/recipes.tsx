import { createFileRoute } from '@tanstack/react-router';

import { RecipePreview } from '@/features/ask';
import { RecipesScreen } from '@/features/recipes';

/**
 * `/recipes`: the Recipes page (AI-04). Preview runs a recipe through the
 * `ask` feature, handed in here so the two features stay independent.
 */
export const Route = createFileRoute('/recipes')({ component: RecipesRoute });

function RecipesRoute() {
  return <RecipesScreen Preview={RecipePreview} />;
}
