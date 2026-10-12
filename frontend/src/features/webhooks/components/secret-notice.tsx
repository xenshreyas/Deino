import { useState } from 'react';

import { Alert } from '@/ui/alert';
import { Button } from '@/ui/button';
import { Check, Copy, ShieldCheck } from '@/ui/icons';
import { IconButton } from '@/ui/icon-button';

import { webhooksCopy } from '../copy';

/** Props of {@link SecretNotice}. */
export interface SecretNoticeProps {
  secret: string;
  onDismiss: () => void;
}

/** A new signing secret, shown this once, with a copy button. */
export function SecretNotice({ secret, onDismiss }: SecretNoticeProps) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    void navigator.clipboard.writeText(secret).then(() => {
      setCopied(true);
    });
  };
  return (
    <Alert variant="info" role="status">
      <ShieldCheck />
      <div className="flex flex-col gap-2">
        <p className="font-medium">{webhooksCopy.secretTitle}</p>
        <div className="flex items-center gap-2">
          <code className="min-w-0 flex-1 truncate rounded-md bg-surface px-2 py-1 font-mono text-xs select-all">
            {secret}
          </code>
          <IconButton
            icon={copied ? Check : Copy}
            label={copied ? webhooksCopy.copied : webhooksCopy.copy}
            onClick={copy}
          />
        </div>
        <p className="text-xs text-fg-muted">{webhooksCopy.secretHint}</p>
        <div>
          <Button size="sm" variant="secondary" onClick={onDismiss}>
            {webhooksCopy.dismiss}
          </Button>
        </div>
      </div>
    </Alert>
  );
}
