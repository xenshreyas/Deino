import { useState } from 'react';

import { Check, Copy } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { cliCopy } from '../copy';

/** Props of {@link Snippet}. */
export interface SnippetProps {
  /** What the snippet is, such as "Claude Code". */
  label: string;
  /** One line of help under the label. */
  hint: string;
  /** The text to show and copy. */
  text: string;
}

/**
 * A labelled block of monospace text with a copy button; the button shows a
 * check once the text is on the clipboard.
 */
export function Snippet({ label, hint, text }: SnippetProps) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
    });
  };
  return (
    <div className="flex flex-col gap-1.5 px-4 py-3">
      <div className="flex items-center justify-between gap-4">
        <div className="min-w-0">
          <p className="text-sm text-fg">{label}</p>
          <p className="mt-0.5 text-xs text-fg-muted">{hint}</p>
        </div>
        <IconButton
          icon={copied ? Check : Copy}
          label={copied ? cliCopy.copied : cliCopy.copy(label)}
          onClick={copy}
        />
      </div>
      <pre className="overflow-x-auto rounded-md bg-bg-muted px-3 py-2 font-mono text-xs text-fg select-all">
        {text}
      </pre>
    </div>
  );
}
