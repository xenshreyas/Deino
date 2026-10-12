/**
 * Holds the app back until the library is open (PRV-02). Most launches open
 * it before the window shows, so this renders the app at once; while the
 * database is encrypted or decrypted it shows a calm progress screen, and
 * when a key problem stops it from opening it explains what to do.
 */
import { useQuery } from '@tanstack/react-query';
import type { ReactNode } from 'react';

import { ICON_PROPS_LARGE, Lock, ShieldCheck } from '@/ui/icons';

import { privacyCopy } from '../copy';
import { libraryStatusQuery } from '../queries';
import type { LibraryStatus } from '../types';

const copy = privacyCopy.library;

/** A centred message on the bare window. */
function Notice({ icon, title, detail }: { icon: ReactNode; title: string; detail: string }) {
  return (
    <div className="flex h-screen w-screen flex-col bg-bg text-fg">
      <div data-tauri-drag-region className="h-11 shrink-0" />
      <main
        role="status"
        className="flex flex-1 flex-col items-center justify-center gap-4 px-6 pb-11 text-center"
      >
        <div className="flex size-12 items-center justify-center rounded-full bg-bg-muted text-fg-muted">
          {icon}
        </div>
        <h1 className="text-base font-semibold">{title}</h1>
        <p className="max-w-sm text-sm text-fg-muted">{detail}</p>
      </main>
    </div>
  );
}

/** The screen for a library that is not open yet, or `null` once it is. */
export function LibraryNotice({ status }: { status: LibraryStatus }) {
  switch (status.kind) {
    case 'ready':
      return null;
    case 'opening':
      return <div className="h-screen w-screen bg-bg" />;
    case 'converting':
      return (
        <Notice
          icon={<ShieldCheck {...ICON_PROPS_LARGE} />}
          title={copy.converting(status.direction)}
          detail={copy.convertingDetail}
        />
      );
    case 'unavailable':
      return (
        <Notice
          icon={<Lock {...ICON_PROPS_LARGE} />}
          title={copy.unavailableTitle}
          detail={copy.problems[status.problem]}
        />
      );
  }
}

/** What shows before Rust's first answer. */
const OPENING: LibraryStatus = { kind: 'opening' };

/** The app, once the library is open. If the status cannot be read at
 * all, the app renders and reports its own errors. */
export function LibraryGate({ children }: { children: ReactNode }) {
  const query = useQuery(libraryStatusQuery());
  if (query.isError || query.data?.kind === 'ready') return children;
  return <LibraryNotice status={query.data ?? OPENING} />;
}
