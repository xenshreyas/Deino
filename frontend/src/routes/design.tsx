import { createFileRoute, notFound } from '@tanstack/react-router';
import { lazy, Suspense } from 'react';

/**
 * The gallery, loaded only in development. Vite replaces
 * `import.meta.env.DEV` with `false` in release builds, so the import (and
 * the `_example` feature it shows) is dropped from the bundle.
 */
const DesignScreen = import.meta.env.DEV
  ? lazy(() =>
      import('@/features/design-system').then((module) => ({ default: module.DesignScreen })),
    )
  : null;

/** `/design`: every token and primitive in both themes (dev builds only). */
export const Route = createFileRoute('/design')({
  beforeLoad: () => {
    if (DesignScreen === null) throw notFound();
  },
  component: DesignRoute,
});

function DesignRoute() {
  if (DesignScreen === null) return null;
  return (
    <Suspense fallback={null}>
      <DesignScreen />
    </Suspense>
  );
}
