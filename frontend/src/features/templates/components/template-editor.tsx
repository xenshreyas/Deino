import { Link, useNavigate } from '@tanstack/react-router';
import { useState } from 'react';

import { Button, buttonVariants } from '@/ui/button';
import { Separator } from '@/ui/controls';
import { ChevronRight, ICON_PROPS, Lock } from '@/ui/icons';
import { SkeletonText } from '@/ui/skeleton';

import { templatesCopy } from '../copy';
import { type Draft, emptyDraft, fromDraft, toDraft } from '../draft';
import { useDuplicateTemplate, useSaveTemplate, useTemplate } from '../queries';
import type { Template, TemplateId } from '../types';
import { EditorFields } from './editor-fields';
import { RuleList } from './rule-list';
import { SectionList } from './section-list';
import { TemplatePreview } from './template-preview';

/** Opens a template in the editor, or the library. */
function useOpenTemplate() {
  const navigate = useNavigate();
  return (template: TemplateId | null) => {
    void navigate({
      to: '/settings',
      search: template === null ? { section: 'templates' } : { section: 'templates', template },
    });
  };
}

/** "‹ Templates", back to the library. */
function BackLink() {
  return (
    <Link
      to="/settings"
      search={{ section: 'templates' }}
      className={buttonVariants({
        variant: 'ghost',
        size: 'sm',
        className: '-ml-2 gap-1 self-start',
      })}
    >
      <ChevronRight {...ICON_PROPS} className="rotate-180" />
      {templatesCopy.back}
    </Link>
  );
}

/** Props of {@link TemplateEditorScreen}. */
export interface TemplateEditorScreenProps {
  /** The template to edit, or `null` for a new one. */
  id: TemplateId | null;
}

/** Loads the template, then edits it; a new template starts blank. */
export function TemplateEditorScreen({ id }: TemplateEditorScreenProps) {
  if (id === null) return <TemplateEditor template={null} />;
  return <LoadedEditor id={id} />;
}

function LoadedEditor({ id }: { id: TemplateId }) {
  const template = useTemplate(id);
  if (template.error !== null) {
    return (
      <p role="alert" className="text-sm text-danger">
        {template.error.message}
      </p>
    );
  }
  if (template.data === undefined) return <SkeletonText lines={6} />;
  return <TemplateEditor key={template.data.id} template={template.data} />;
}

/** The editor of one template (or a new one), with its save bar. */
function TemplateEditor({ template }: { template: Template | null }) {
  const [draft, setDraft] = useState<Draft>(() =>
    template === null ? emptyDraft(templatesCopy.newSection) : toDraft(template.content),
  );
  const readOnly = template?.isBuiltin ?? false;
  const change = (patch: Partial<Draft>) => {
    setDraft((current) => ({ ...current, ...patch }));
  };
  return (
    <div className="flex flex-col gap-6 pb-4">
      <BackLink />
      {template !== null && readOnly ? <ReadOnlyNotice template={template} /> : null}
      <EditorFields draft={draft} readOnly={readOnly} onChange={change} />
      <SectionList
        sections={draft.sections}
        readOnly={readOnly}
        onChange={(sections) => {
          change({ sections });
        }}
      />
      <RuleList
        rules={draft.rules}
        readOnly={readOnly}
        onChange={(rules) => {
          change({ rules });
        }}
      />
      <Separator />
      <TemplatePreview content={() => fromDraft(draft)} />
      {readOnly ? null : <SaveBar template={template} draft={draft} />}
    </div>
  );
}

/** Built-ins cannot change; duplicating makes an editable copy. */
function ReadOnlyNotice({ template }: { template: Template }) {
  const duplicate = useDuplicateTemplate();
  const open = useOpenTemplate();
  return (
    <div className="flex items-center gap-3 rounded-lg bg-bg-subtle px-4 py-3">
      <Lock {...ICON_PROPS} className="shrink-0 text-fg-muted" />
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-sm font-semibold text-fg">{templatesCopy.readOnlyTitle}</span>
        <span className="text-xs text-fg-muted">{templatesCopy.readOnlyBody}</span>
      </div>
      <Button
        variant="secondary"
        disabled={duplicate.isPending}
        onClick={() => {
          duplicate.mutate(template.id, {
            onSuccess: (copy) => {
              open(copy.id);
            },
          });
        }}
      >
        {templatesCopy.duplicate}
      </Button>
    </div>
  );
}

/** The sticky footer: the save error, if any, and Save. */
function SaveBar({ template, draft }: { template: Template | null; draft: Draft }) {
  const save = useSaveTemplate();
  const open = useOpenTemplate();
  const submit = () => {
    save.mutate(
      { id: template?.id ?? null, content: fromDraft(draft) },
      {
        onSuccess: (saved) => {
          if (template === null) open(saved.id);
        },
      },
    );
  };
  return (
    <div className="sticky bottom-0 -mx-6 flex items-center justify-end gap-3 border-t border-border bg-bg px-6 py-3">
      {save.error === null ? null : (
        <p role="alert" className="mr-auto text-sm text-danger">
          {save.error.message}
        </p>
      )}
      <Button disabled={save.isPending || draft.name.trim() === ''} onClick={submit}>
        {save.isPending ? templatesCopy.saving : templatesCopy.save}
      </Button>
    </div>
  );
}
