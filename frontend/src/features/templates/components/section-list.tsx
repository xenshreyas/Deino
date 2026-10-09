import { useState } from 'react';

import { Button } from '@/ui/button';
import { ICON_PROPS, Plus } from '@/ui/icons';

import { templatesCopy } from '../copy';
import { type DraftSection, move, newSection, patchItem } from '../draft';
import { SectionCard, type SectionCardActions } from './section-card';

/** Props of {@link SectionList}. */
export interface SectionListProps {
  sections: DraftSection[];
  readOnly: boolean;
  onChange: (sections: DraftSection[]) => void;
}

/** Which card is being dragged, and which one it is over. */
interface DragState {
  from: number;
  over: number;
}

/** The handlers of the card at `position`. */
function cardActions(
  { sections, onChange }: SectionListProps,
  position: number,
  drag: { state: DragState | null; set: (state: DragState | null) => void },
): SectionCardActions {
  const section = sections[position];
  const key = section?.key ?? '';
  return {
    onChange: (patch) => {
      onChange(patchItem(sections, key, patch));
    },
    onMove: (offset) => {
      onChange(move(sections, position, position + offset));
    },
    onRemove: () => {
      onChange(sections.filter((candidate) => candidate.key !== key));
    },
    onDragStart: () => {
      drag.set({ from: position, over: position });
    },
    onDragOver: () => {
      if (drag.state !== null && drag.state.over !== position)
        drag.set({ ...drag.state, over: position });
    },
    onDrop: () => {
      if (drag.state !== null) onChange(move(sections, drag.state.from, position));
      drag.set(null);
    },
  };
}

/**
 * The template's sections in order: reorder by dragging a card onto another,
 * with the handle's ↑/↓ keys, Alt ↑/↓ from inside a card, or the arrow buttons.
 */
export function SectionList(props: SectionListProps) {
  const { sections, readOnly, onChange } = props;
  const [state, set] = useState<DragState | null>(null);
  return (
    <section aria-labelledby="template-sections" className="flex flex-col gap-2">
      <SectionsHeading />
      <ol
        className="divide-y divide-border rounded-lg border border-border bg-surface"
        onDragEnd={() => {
          set(null);
        }}
      >
        {sections.map((section, position) => (
          <SectionCard
            key={section.key}
            section={section}
            position={position}
            count={sections.length}
            readOnly={readOnly}
            dragging={state?.from === position}
            dropTarget={state !== null && state.over === position && state.from !== position}
            {...cardActions(props, position, { state, set })}
          />
        ))}
      </ol>
      {readOnly ? null : (
        <AddSection
          onAdd={() => {
            onChange([...sections, newSection(templatesCopy.newSection)]);
          }}
        />
      )}
    </section>
  );
}

function AddSection({ onAdd }: { onAdd: () => void }) {
  return (
    <Button variant="ghost" size="sm" className="self-start" onClick={onAdd}>
      <Plus {...ICON_PROPS} />
      {templatesCopy.addSection}
    </Button>
  );
}

function SectionsHeading() {
  return (
    <div className="flex flex-col gap-0.5">
      <h3 id="template-sections" className="text-sm font-semibold text-fg">
        {templatesCopy.sections}
      </h3>
      <p className="text-xs text-fg-muted">{templatesCopy.sectionsHint}</p>
    </div>
  );
}
