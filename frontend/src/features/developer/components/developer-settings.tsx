import { lazy, Suspense } from 'react';

import { CliSetup } from './cli-setup';

/** The sample meetings panel, loaded lazily in debug builds only. */
const SampleMeetings = import.meta.env.DEV
  ? lazy(() => import('./sample-meetings').then((module) => ({ default: module.SampleMeetings })))
  : null;

/**
 * Settings → Developer: the command-line tool and Claude setup, and in
 * debug builds the sample meetings.
 */
export function DeveloperSettings() {
  return (
    <div className="flex flex-col gap-8">
      <CliSetup />
      {SampleMeetings === null ? null : (
        <Suspense fallback={null}>
          <SampleMeetings />
        </Suspense>
      )}
    </div>
  );
}
