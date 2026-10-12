/**
 * A comment field with its send button: ⌘↵ sends, Esc cancels. Used to
 * start a thread and to reply.
 */
import { type FormEvent, type KeyboardEvent, useState } from 'react';

import { Button } from '@/ui/button';
import { Textarea } from '@/ui/input';

import { commentsCopy } from '../copy';

/** Props of {@link CommentBox}. */
export interface CommentBoxProps {
  placeholder: string;
  submitLabel: string;
  busy: boolean;
  autoFocus?: boolean;
  /** Called with the text; resolve to clear the field. */
  onSubmit: (body: string) => Promise<unknown>;
  onCancel?: () => void;
}

/** The field's text, sending and the keys. */
function useCommentField({ busy, onSubmit, onCancel }: CommentBoxProps) {
  const [body, setBody] = useState('');
  const empty = body.trim() === '';
  const send = () => {
    if (empty || busy) return;
    void onSubmit(body).then(() => {
      setBody('');
    });
  };
  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) send();
    if (event.key === 'Escape') onCancel?.();
  };
  return { body, setBody, empty, send, onKeyDown };
}

/** The field and its buttons. */
export function CommentBox(props: CommentBoxProps) {
  const { body, setBody, empty, send, onKeyDown } = useCommentField(props);
  const onSubmit = (event: FormEvent) => {
    event.preventDefault();
    send();
  };
  return (
    <form className="flex flex-col gap-2" onSubmit={onSubmit}>
      <Textarea
        autoFocus={props.autoFocus}
        rows={2}
        value={body}
        placeholder={props.placeholder}
        aria-label={props.placeholder}
        onChange={(event) => {
          setBody(event.target.value);
        }}
        onKeyDown={onKeyDown}
        className="min-h-0 resize-none"
      />
      <div className="flex justify-end gap-1.5">
        {props.onCancel === undefined ? null : (
          <Button type="button" size="sm" variant="ghost" onClick={props.onCancel}>
            {commentsCopy.cancel}
          </Button>
        )}
        <Button type="submit" size="sm" disabled={empty || props.busy}>
          {props.submitLabel}
        </Button>
      </div>
    </form>
  );
}
