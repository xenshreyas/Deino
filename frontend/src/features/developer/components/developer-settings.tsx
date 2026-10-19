import { lazy, Suspense } from 'react';

import { LocalApiSettings } from '@/features/local-api';
import { WebhookSettings } from '@/features/webhooks';

import { CallWindowCapture } from './call-window-capture';
import { CliSetup } from './cli-setup';

/** The sample meetings panel, loaded lazily in debug builds only. */
const SampleMeetings = import.meta.env.DEV
  ? lazy(() => import('./sample-meetings').then((module) => ({ default: module.SampleMeetings })))
  : null;

/**
 * Settings → Developer: the command-line tool and Claude setup, the local
 * REST API and its keys, webhooks (DEV-01), diagnostics (the call-window
 * dumps of CAP-05), and in debug builds the sample meetings.
 */
export function DeveloperSettings() {
  return (
    <div className="flex flex-col gap-8">
      <CliSetup />
      <LocalApiSettings />
      <WebhookSettings />
      <CallWindowCapture />
      {SampleMeetings === null ? null : (
        <Suspense fallback={null}>
          <SampleMeetings />
        </Suspense>
      )}
    </div>
  );
}
