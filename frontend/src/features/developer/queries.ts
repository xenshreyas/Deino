/**
 * TanStack Query hooks for Settings → Developer's command-line setup.
 *
 * The CLI's install state lives on disk, not in the database, so it is
 * re-read whenever the page mounts and an install writes its result straight
 * into the cache. The debug-only sample seeding is in `sample-queries.ts`.
 */
import { queryOptions, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { developerApi } from './api';
import type { InstallDir } from './types';

/** Query keys; every key starts with `developerKeys.all`. */
export const developerKeys = {
  all: ['developer'] as const,
  cli: () => [...developerKeys.all, 'cli'] as const,
};

/** The CLI's setup; re-read on each mount. */
export const cliStatusQuery = () =>
  queryOptions({ queryKey: developerKeys.cli(), queryFn: developerApi.cliStatus, staleTime: 0 });

/** Where the bundled CLI is, where it is installed, and the Claude snippets. */
export function useCliStatus() {
  return useQuery(cliStatusQuery());
}

/** Installs the CLI into a folder and caches the new status. */
export function useInstallCli() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (dir: InstallDir) => developerApi.installCli(dir),
    onSuccess: (status) => {
      queryClient.setQueryData(developerKeys.cli(), status);
    },
  });
}
