/**
 * The icons a folder can wear. Rust stores the name (`folders.icon`); a name
 * this build does not know shows the default folder, so the set can grow.
 */
import {
  BookOpen,
  Briefcase,
  Building2,
  Code,
  Flag,
  Folder,
  GraduationCap,
  Heart,
  type Icon,
  Lightbulb,
  Megaphone,
  Rocket,
  Sparkles,
  Star,
  Target,
  Users,
} from '@/ui/icons';

/** One choice in the icon picker. */
export interface FolderIconChoice {
  /** The stored name: lower-case letters, digits and dashes. */
  name: string;
  /** What it is called, for screen readers and tooltips. */
  label: string;
  icon: Icon;
}

/** Every icon, in picker order; the first is the default. */
export const FOLDER_ICONS: readonly FolderIconChoice[] = [
  { name: 'folder', label: 'Folder', icon: Folder },
  { name: 'briefcase', label: 'Briefcase', icon: Briefcase },
  { name: 'users', label: 'People', icon: Users },
  { name: 'building', label: 'Company', icon: Building2 },
  { name: 'rocket', label: 'Launch', icon: Rocket },
  { name: 'target', label: 'Goal', icon: Target },
  { name: 'star', label: 'Star', icon: Star },
  { name: 'flag', label: 'Flag', icon: Flag },
  { name: 'lightbulb', label: 'Idea', icon: Lightbulb },
  { name: 'sparkles', label: 'Sparkles', icon: Sparkles },
  { name: 'code', label: 'Engineering', icon: Code },
  { name: 'book', label: 'Learning', icon: BookOpen },
  { name: 'graduation-cap', label: 'Hiring', icon: GraduationCap },
  { name: 'megaphone', label: 'Marketing', icon: Megaphone },
  { name: 'heart', label: 'Personal', icon: Heart },
];

/** The icon stored as `name`, or the default folder. */
export function folderIcon(name: string | null): Icon {
  return FOLDER_ICONS.find((choice) => choice.name === name)?.icon ?? Folder;
}
