/** One quiet icon per kind of step, shared by the picker and the step list. */
import {
  Bell,
  Building2,
  Code,
  FileDown,
  FileText,
  FolderInput,
  Globe,
  type Icon,
  Lightbulb,
  ListChecks,
  Mail,
  MessageCircle,
  NotebookText,
  Tag,
} from '@/ui/icons';

import type { ActionKind } from '../types';

const STEP_ICONS: Record<ActionKind, Icon> = {
  generate_insight: Lightbulb,
  add_tag: Tag,
  add_to_folder: FolderInput,
  export_file: FileDown,
  notify: Bell,
  run_shell: Code,
  webhook: Globe,
  slack: MessageCircle,
  notion: NotebookText,
  tasks: ListChecks,
  crm: Building2,
  document: FileText,
  email: Mail,
};

/** The icon of a kind of step. */
export function stepIcon(kind: ActionKind): Icon {
  return STEP_ICONS[kind];
}
