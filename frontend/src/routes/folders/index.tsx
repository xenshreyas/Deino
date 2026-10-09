import { createFileRoute } from '@tanstack/react-router';

import { folderTreeQuery, FoldersScreen } from '@/features/folders';

/** `/folders`: every folder, and Unfiled. */
export const Route = createFileRoute('/folders/')({
  loader: async ({ context }) => {
    await context.queryClient.query(folderTreeQuery()).catch(() => undefined);
  },
  component: FoldersScreen,
});
