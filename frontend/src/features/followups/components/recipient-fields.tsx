/**
 * To, Cc and Bcc, as in a mail app: one row each, label on the left. Cc and
 * Bcc stay folded behind two quiet buttons until used or already filled.
 */
import { type ReactNode, useState } from 'react';

import { Button } from '@/ui/button';
import { Input } from '@/ui/input';

import { followUpsCopy } from '../copy';
import type { DraftFields } from '../types';

const copy = followUpsCopy.editor;

/** The address fields of the draft. */
type AddressKey = 'to' | 'cc' | 'bcc';

/** Props of {@link RecipientFields}. */
export interface RecipientFieldsProps {
  fields: DraftFields;
  onChange: (key: AddressKey, value: string) => void;
}

/** Props of one {@link AddressRow}. */
interface AddressRowProps {
  field: AddressKey;
  label: string;
  value: string;
  onChange: (key: AddressKey, value: string) => void;
  trailing?: ReactNode;
}

/** One labelled address row. */
function AddressRow({ field, label, value, onChange, trailing }: AddressRowProps) {
  const id = `follow-up-${field}`;
  return (
    <div className="flex items-center gap-2">
      <label htmlFor={id} className="w-10 shrink-0 text-xs text-fg-muted">
        {label}
      </label>
      <Input
        id={id}
        value={value}
        placeholder={copy.addressHint}
        autoComplete="off"
        spellCheck={false}
        onChange={(event) => {
          onChange(field, event.target.value);
        }}
      />
      {trailing}
    </div>
  );
}

/** A quiet button that unfolds a hidden row; nothing once it is shown. */
function Unfold({ shown, label, onShow }: { shown: boolean; label: string; onShow: () => void }) {
  if (shown) return null;
  return (
    <Button variant="ghost" size="sm" onClick={onShow}>
      {label}
    </Button>
  );
}

/** To, then Cc and Bcc when shown. */
export function RecipientFields({ fields, onChange }: RecipientFieldsProps) {
  const [showCc, setShowCc] = useState(fields.cc !== '');
  const [showBcc, setShowBcc] = useState(fields.bcc !== '');
  const toggles = (
    <div className="flex shrink-0">
      <Unfold
        shown={showCc}
        label={copy.cc}
        onShow={() => {
          setShowCc(true);
        }}
      />
      <Unfold
        shown={showBcc}
        label={copy.bcc}
        onShow={() => {
          setShowBcc(true);
        }}
      />
    </div>
  );
  return (
    <div className="flex flex-col gap-2">
      <AddressRow
        field="to"
        label={copy.to}
        value={fields.to}
        onChange={onChange}
        trailing={toggles}
      />
      {showCc ? (
        <AddressRow field="cc" label={copy.cc} value={fields.cc} onChange={onChange} />
      ) : null}
      {showBcc ? (
        <AddressRow field="bcc" label={copy.bcc} value={fields.bcc} onChange={onChange} />
      ) : null}
    </div>
  );
}
