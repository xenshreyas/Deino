/**
 * "More options": the closed disclosure under a settings section's one
 * primary control (UX-15, progressive disclosure). What one user in twenty
 * changes sits inside; the everyday control stays outside it.
 */
import type { ReactNode } from 'react';

import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from './accordion';

/** Props of {@link MoreOptions}. */
export interface MoreOptionsProps {
  /** The trigger's words; "More options" unless the section needs a clearer one. */
  label?: string;
  children: ReactNode;
}

/** A closed "More options" disclosure; its content mounts only when opened. */
export function MoreOptions({ label = 'More options', children }: MoreOptionsProps) {
  return (
    <Accordion type="single" collapsible>
      <AccordionItem value="more" className="border-b-0">
        <AccordionTrigger className="w-fit justify-start gap-1.5 py-1.5 text-xs font-normal text-fg-muted hover:text-fg">
          {label}
        </AccordionTrigger>
        <AccordionContent className="flex flex-col gap-4 pt-2 pb-0">{children}</AccordionContent>
      </AccordionItem>
    </Accordion>
  );
}
