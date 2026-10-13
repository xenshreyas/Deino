/** The brand on the `/design` page: the wordmark and the mark at their working sizes. */
import { BrandMark, Wordmark } from '@/ui/brand';

const MARK_SIZES = [16, 24, 44] as const;
const WORDMARK_SIZES = [14, 20] as const;

/** Both brand components, small to large, with the token each one paints. */
export function BrandGallery() {
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-end gap-6">
        {MARK_SIZES.map((size) => (
          <BrandMark key={size} size={size} />
        ))}
        <span className="text-xs text-fg-muted">BrandMark · --brand</span>
      </div>
      <div className="flex items-end gap-6">
        {WORDMARK_SIZES.map((size) => (
          <Wordmark key={size} size={size} />
        ))}
        <span className="text-xs text-fg-muted">Wordmark · --fg</span>
      </div>
      <div className="flex items-center gap-2">
        <BrandMark size={20} />
        <Wordmark size={16} />
      </div>
    </div>
  );
}
