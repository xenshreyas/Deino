import { useNavigate } from '@tanstack/react-router';
import { type ReactNode, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/ui/button';
import { ICON_PROPS, LayoutTemplate, Plus, Upload } from '@/ui/icons';
import { SkeletonText } from '@/ui/skeleton';

import { templatesApi } from '../api';
import { templatesCopy } from '../copy';
import { useDuplicateTemplate, useTemplates } from '../queries';
import type { Template, TemplateId } from '../types';
import { DefaultTemplate } from './default-template';
import { DeleteTemplateDialog, ImportTemplateDialog } from './template-dialogs';
import { TemplateRow, type TemplateRowActions } from './template-row';

/** Opens a template (or a new one, `new`) in the editor. */
function useOpen() {
  const navigate = useNavigate();
  return (template: TemplateId | 'new') => {
    void navigate({ to: '/settings', search: { section: 'templates', template } });
  };
}

/** Copies a template's file to the clipboard. */
async function exportTemplate(template: Template) {
  const json = await templatesApi.exportJson(template.id);
  await navigator.clipboard.writeText(json);
  toast.success(templatesCopy.exported);
}

/** The rows' actions and the dialogs they open. */
function useLibraryActions() {
  const duplicate = useDuplicateTemplate();
  const open = useOpen();
  const [deleting, setDeleting] = useState<Template | null>(null);
  const [importing, setImporting] = useState(false);
  const actions: TemplateRowActions = {
    onDuplicate: (template) => {
      duplicate.mutate(template.id, {
        onSuccess: (copy) => {
          open(copy.id);
        },
      });
    },
    onExport: (template) => {
      exportTemplate(template).catch((error: unknown) => toast.error(String(error)));
    },
    onDelete: setDeleting,
  };
  return { actions, open, deleting, setDeleting, importing, setImporting };
}

/**
 * Settings → Templates: the default template, then the user's templates and
 * the built-ins, each row opening the editor.
 */
export function TemplateLibrary() {
  const templates = useTemplates();
  const library = useLibraryActions();
  if (templates.data === undefined) return <SkeletonText lines={8} />;
  const create = () => {
    library.open('new');
  };
  return (
    <div className="flex flex-col gap-8">
      <DefaultTemplate templates={templates.data} />
      <Toolbar
        onNew={create}
        onImport={() => {
          library.setImporting(true);
        }}
      />
      <Groups templates={templates.data} actions={library.actions} onNew={create} />
      <LibraryDialogs {...library} />
    </div>
  );
}

/** The delete confirmation and the import dialog. */
function LibraryDialogs(library: ReturnType<typeof useLibraryActions>) {
  return (
    <>
      <DeleteTemplateDialog
        template={library.deleting}
        onClose={() => {
          library.setDeleting(null);
        }}
      />
      <ImportTemplateDialog
        open={library.importing}
        onClose={() => {
          library.setImporting(false);
        }}
        onImported={(template) => {
          library.setImporting(false);
          library.open(template.id);
        }}
      />
    </>
  );
}

function Toolbar({ onNew, onImport }: { onNew: () => void; onImport: () => void }) {
  return (
    <div className="-mb-6 flex items-center justify-end gap-2">
      <Button variant="ghost" size="sm" onClick={onImport}>
        <Upload {...ICON_PROPS} />
        {templatesCopy.import}
      </Button>
      <Button variant="secondary" size="sm" onClick={onNew}>
        <Plus {...ICON_PROPS} />
        {templatesCopy.newTemplate}
      </Button>
    </div>
  );
}

interface GroupsProps {
  templates: Template[];
  actions: TemplateRowActions;
  onNew: () => void;
}

/** "Your templates" (or its empty state), then "Built-in". */
function Groups({ templates, actions, onNew }: GroupsProps) {
  const yours = templates.filter((template) => !template.isBuiltin);
  const builtIn = templates.filter((template) => template.isBuiltin);
  return (
    <>
      <Group title={templatesCopy.yours} templates={yours} actions={actions}>
        <NoTemplatesYet onNew={onNew} />
      </Group>
      <Group title={templatesCopy.builtIn} templates={builtIn} actions={actions} />
    </>
  );
}

/** The resting state of "Your templates", with its one action. */
function NoTemplatesYet({ onNew }: { onNew: () => void }) {
  return (
    <div className="flex items-center gap-3 rounded-lg border border-dashed border-border px-4 py-4">
      <LayoutTemplate {...ICON_PROPS} className="shrink-0 text-fg-muted" />
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-sm text-fg">{templatesCopy.emptyTitle}</span>
        <span className="text-xs text-fg-muted">{templatesCopy.emptyBody}</span>
      </div>
      <Button variant="secondary" size="sm" onClick={onNew}>
        {templatesCopy.newTemplate}
      </Button>
    </div>
  );
}

interface GroupProps {
  title: string;
  templates: Template[];
  actions: TemplateRowActions;
  /** Shown instead of the list when it is empty. */
  children?: ReactNode;
}

/** A titled list of rows, or `children` when it has none. */
function Group({ title, templates, actions, children }: GroupProps) {
  const id = `templates-${title.toLowerCase().replace(/\W+/g, '-')}`;
  return (
    <section aria-labelledby={id} className="flex flex-col gap-1">
      <h3 id={id} className="flex h-6 items-center text-xs font-semibold text-fg-muted">
        {title}
      </h3>
      {templates.length === 0 ? (
        children
      ) : (
        <ul className="-mx-3 flex flex-col">
          {templates.map((template) => (
            <TemplateRow key={template.id} template={template} {...actions} />
          ))}
        </ul>
      )}
    </section>
  );
}
