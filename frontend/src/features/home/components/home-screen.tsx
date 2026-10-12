/**
 * Home (ORG-02, ui.md §4.1): a greeting, Coming up with Join and the record
 * toggle, Recent notes, then suggested follow-ups (NOT-07's slot, hidden
 * while empty) beside the open action items due soon (AI-05).
 */
import { Link } from '@tanstack/react-router';

import { TodayDigest } from '@/features/action-items';

import { homeCopy } from '../copy';
import { HOME_SLOTS, type SuggestedFollowUp } from '../slots';
import { ComingUpSection } from './coming-up-section';
import { RecentNotes } from './recent-notes';
import { SectionHeading } from './section-heading';

/** NOT-07's suggestions; nothing at all while there are none. */
function FollowUps({ items }: { items: readonly SuggestedFollowUp[] }) {
  if (items.length === 0) return null;
  return (
    <section aria-labelledby="home-follow-ups" className="flex min-w-0 flex-1 flex-col gap-1">
      <SectionHeading id="home-follow-ups">{homeCopy.followUps}</SectionHeading>
      <ul>
        {items.map((item) => (
          <li key={item.id}>
            <Link
              to="/meetings/$meetingId"
              params={{ meetingId: item.meetingId }}
              className="flex h-10 items-center rounded-md px-3 text-sm text-fg hover:bg-bg-muted"
            >
              {item.title}
            </Link>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** Home's page body. */
export function HomeScreen() {
  const followUps = HOME_SLOTS.useFollowUps();
  return (
    <div className="mx-auto flex w-full max-w-[960px] flex-col gap-10 px-6 pt-12 pb-28">
      <h1 className="px-3 text-2xl font-semibold text-fg">
        {homeCopy.greeting(new Date().getHours())}
      </h1>
      <ComingUpSection />
      <RecentNotes />
      <div className="flex flex-col gap-10 md:flex-row">
        <FollowUps items={followUps} />
        <section aria-label={homeCopy.actionItems} className="min-w-0 flex-1">
          <TodayDigest hideWhenEmpty limit={5} />
        </section>
      </div>
    </div>
  );
}
