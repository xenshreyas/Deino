/**
 * The settings section shape (ui.md §3 Settings layout): a title and short
 * description outside a hairline card whose rows are divided by hairlines.
 */
import type { ReactNode } from 'react';

/** Props of {@link SettingsSection}. */
export interface SettingsSectionProps {
  title: string;
  description: string;
  children: ReactNode;
}

/** A titled section; put {@link SettingsCard}s or a status line inside. */
export function SettingsSection({ title, description, children }: SettingsSectionProps) {
  return (
    <section className="flex flex-col gap-2">
      <div>
        <h3 className="text-sm font-semibold text-fg">{title}</h3>
        <p className="mt-0.5 text-xs text-fg-muted">{description}</p>
      </div>
      {children}
    </section>
  );
}

/** The card that holds a section's rows. */
export function SettingsCard({ children, label }: { children: ReactNode; label?: string }) {
  return (
    <div
      role="group"
      aria-label={label}
      className="divide-y divide-border rounded-lg border border-border bg-surface"
    >
      {children}
    </div>
  );
}
