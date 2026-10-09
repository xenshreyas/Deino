import { createFileRoute } from '@tanstack/react-router';

import { asFolderId, FolderScreen, folderQuery } from '@/features/folders';

/** `/folders/$folderId`: one folder's meetings, description and rules. */
export const Route = createFileRoute('/folders/$folderId')({
  // Warm the header's query; the page shows a missing folder itself.
  loader: async ({ context, params }) => {
    await context.queryClient
      .query(folderQuery(asFolderId(params.folderId)))
      .catch(() => undefined);
  },
  component: FolderRoute,
});

function FolderRoute() {
  const { folderId } = Route.useParams();
  // Remount per folder so in-place edits start from that folder's values.
  return <FolderScreen key={folderId} folderId={asFolderId(folderId)} />;
}
