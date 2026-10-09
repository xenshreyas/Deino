import { createFileRoute } from '@tanstack/react-router';

import { tagListQuery, TagsScreen } from '@/features/tags';

/** `/tags`: every tag, and the auto-tagging switch (ORG-06). */
export const Route = createFileRoute('/tags')({
  loader: async ({ context }) => {
    await context.queryClient.query(tagListQuery()).catch(() => undefined);
  },
  component: TagsScreen,
});
