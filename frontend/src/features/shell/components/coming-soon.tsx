import { Link } from '@tanstack/react-router';

import { buttonVariants } from '@/ui/button';
import { EmptyState } from '@/ui/empty-state';

import { shellCopy } from '../copy';
import { navItem, type NavItemId } from '../nav';

/**
 * The calm placeholder for a sidebar destination whose feature card has not
 * shipped yet: what it will do, and a way back.
 */
export function ComingSoonScreen({ navId }: { navId: NavItemId }) {
  const item = navItem(navId);
  return (
    <div className="h-full overflow-y-auto">
      <EmptyState
        icon={item.icon}
        title={shellCopy.comingSoonTitle(item.label)}
        description={item.comingSoon?.description}
        action={
          <Link to="/" className={buttonVariants({ variant: 'secondary' })}>
            {shellCopy.comingSoonAction}
          </Link>
        }
      />
    </div>
  );
}
