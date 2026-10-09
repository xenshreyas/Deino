/**
 * The `/template` command (NOT-04's `features/templates/slash`): typing
 * `/template` lists the templates; choosing one sets the meeting's template
 * and inserts its section headings to write under. The notepad registers
 * the self-contained suggestion here and draws `TemplateSlashList` as its
 * menu (`components/template-menu.tsx`).
 */
import { Extension } from '@tiptap/core';
import { PluginKey } from '@tiptap/pm/state';
import Suggestion from '@tiptap/suggestion';

import {
  type Template,
  type TemplateSlashDeps,
  templateSlashSuggestion,
} from '@/features/templates';

import { type MenuStore, suggestionRenderer } from './menu-store';

/** The plugin's own key, apart from the mention suggestion's. */
const TEMPLATE_COMMAND_KEY = new PluginKey('templateCommand');

/** Options of {@link TemplateCommand}; without both, `/template` does nothing. */
export interface TemplateCommandOptions {
  menu: MenuStore<Template> | null;
  deps: TemplateSlashDeps | null;
}

/** What the suggestion plugin hands the menu. */
interface TemplateRenderProps {
  items: Template[];
  clientRect?: (() => DOMRect | null) | null;
  command: (template: Template) => void;
}

/** The `/template` suggestion, wired to the menu's store. */
export const TemplateCommand = Extension.create<TemplateCommandOptions>({
  name: 'templateCommand',

  addOptions() {
    return { menu: null, deps: null };
  },

  addProseMirrorPlugins() {
    const { menu, deps } = this.options;
    if (menu === null || deps === null) return [];
    return [
      Suggestion<Template, Template>({
        editor: this.editor,
        pluginKey: TEMPLATE_COMMAND_KEY,
        ...templateSlashSuggestion(deps),
        render: () =>
          suggestionRenderer<Template, TemplateRenderProps>(menu, (props, template) => {
            props.command(template);
          }),
      }),
    ];
  },
});
