import { Button } from '@/ui/button';
import { DatabaseZap, ICON_PROPS } from '@/ui/icons';

import { copy } from '../copy';
import { useSeedFixtures } from '../sample-queries';

/**
 * Settings → Developer's sample meetings (debug builds only): load the fixture meetings into
 * the database, then report what was written or why it failed.
 */
export function SampleMeetings() {
  const seed = useSeedFixtures();

  return (
    <section className="flex flex-col gap-3 rounded-lg border border-border bg-surface p-5 text-fg">
      <h2 className="flex items-center gap-2 text-sm font-semibold">
        <DatabaseZap {...ICON_PROPS} />
        {copy.title}
      </h2>
      <p className="text-sm text-fg-muted">{copy.description}</p>
      <div className="flex items-center gap-3">
        <Button
          variant="secondary"
          disabled={seed.isPending}
          onClick={() => {
            seed.mutate();
          }}
        >
          {seed.isPending ? copy.seeding : copy.seed}
        </Button>
        <SeedOutcome seed={seed} />
      </div>
    </section>
  );
}

/** The last seed's result: a count on success, the error message on failure. */
function SeedOutcome({ seed }: { seed: ReturnType<typeof useSeedFixtures> }) {
  if (seed.data) {
    return (
      <p role="status" className="text-sm text-fg-muted">
        {copy.seeded(seed.data.meetings, seed.data.segments)}
      </p>
    );
  }
  if (seed.error) {
    return (
      <p role="alert" className="text-sm text-danger">
        {seed.error.message}
      </p>
    );
  }
  return null;
}
