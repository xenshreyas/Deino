import { useSearch } from '@tanstack/react-router';

import { asTemplateId } from '../types';
import { TemplateEditorScreen } from './template-editor';
import { TemplateLibrary } from './template-library';

/**
 * Settings → Templates. The open template lives in the URL
 * (`/settings?section=templates&template=<id>`, `new` for a new one), so the
 * back button and links work; without it, the library.
 */
export function TemplatesPage() {
  const { template } = useSearch({ from: '/settings' });
  if (template === undefined) return <TemplateLibrary />;
  return (
    <TemplateEditorScreen key={template} id={template === 'new' ? null : asTemplateId(template)} />
  );
}
