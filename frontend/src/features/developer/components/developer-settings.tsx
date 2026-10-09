import { DatabaseZap, ICON_PROPS } from '@/ui/icons';

import { copy } from '../copy';
import { useSeedFixtures } from '../queries';

/**
 * Settings → Developer (debug builds only): load the fixture meetings into
 * the database, then report what was written or why it failed.
 */
export function DeveloperSettings() {
  const seed = useSeedFixtures();

  return (
    <section className="mt-6 flex max-w-[720px] flex-col gap-3 rounded-lg border border-border bg-card p-6 text-card-foreground">
      <h2 className="flex items-center gap-2 text-base font-semibold">
        <DatabaseZap {...ICON_PROPS} />
        {copy.title}
      </h2>
      <p className="text-sm text-muted-foreground">{copy.description}</p>
      <div className="flex items-center gap-3">
        <button
          type="button"
          className="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground disabled:opacity-50"
          disabled={seed.isPending}
          onClick={() => {
            seed.mutate();
          }}
        >
          {seed.isPending ? copy.seeding : copy.seed}
        </button>
        <SeedOutcome seed={seed} />
      </div>
    </section>
  );
}

/** The last seed's result: a count on success, the error message on failure. */
function SeedOutcome({ seed }: { seed: ReturnType<typeof useSeedFixtures> }) {
  if (seed.data) {
    return (
      <p role="status" className="text-sm text-muted-foreground">
        {copy.seeded(seed.data.meetings, seed.data.segments)}
      </p>
    );
  }
  if (seed.error) {
    return (
      <p role="alert" className="text-sm text-destructive">
        {seed.error.message}
      </p>
    );
  }
  return null;
}
