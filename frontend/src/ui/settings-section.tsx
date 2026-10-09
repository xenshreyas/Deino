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

/** Props of {@link SettingsRow}. */
export interface SettingsRowProps {
  label: string;
  /** At most two lines of explanation under the label. */
  description?: string;
  /** The id of the control, so the label names it. */
  htmlFor?: string;
  /** The control on the right: a switch, a 200px input or a button. */
  children: ReactNode;
}

/** One setting: label and description on the left, its control on the right. */
export function SettingsRow({ label, description, htmlFor, children }: SettingsRowProps) {
  return (
    <div className="grid min-h-[52px] grid-cols-[1fr_auto] items-center gap-4 px-4 py-3">
      <div className="min-w-0">
        <label htmlFor={htmlFor} className="text-sm text-fg">
          {label}
        </label>
        {description === undefined ? null : (
          <p className="mt-0.5 line-clamp-2 text-xs text-fg-muted">{description}</p>
        )}
      </div>
      {children}
    </div>
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
