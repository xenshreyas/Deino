/**
 * The period's loading shape: four KPI tiles, then two columns of rows.
 */
import { Skeleton } from '@/ui/skeleton';

/** Placeholder blocks in the shape of the period sections. */
export function StatsSkeleton() {
  return (
    <div role="status" aria-label="Loading" className="flex flex-col gap-10">
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        {Array.from({ length: 4 }, (_, index) => (
          <div key={index} className="flex flex-col gap-2">
            <Skeleton className="h-2.5 w-1/2" />
            <Skeleton className="h-6 w-1/3" />
          </div>
        ))}
      </div>
      <div className="grid gap-10 md:grid-cols-2">
        {Array.from({ length: 2 }, (_, column) => (
          <div key={column} className="flex flex-col gap-3">
            {Array.from({ length: 5 }, (_, row) => (
              <Skeleton key={row} className="h-2.5 w-[76%]" />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
