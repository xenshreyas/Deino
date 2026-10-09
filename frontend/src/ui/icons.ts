/**
 * The app's icons, the only module that imports an icon library (ESLint
 * enforces it). Lucide at 16 px with a 1.5 stroke is the house style (§3B);
 * add an icon here before using it, and spread {@link ICON_PROPS} on it.
 */
import type { LucideIcon } from 'lucide-react';

export {
  ArrowLeft,
  AudioLines,
  Building2,
  Calendar,
  Check,
  ChevronDown,
  ChevronRight,
  CircleAlert,
  CircleCheck,
  Clock,
  Copy,
  Cpu,
  DatabaseZap,
  Ellipsis,
  FileText,
  FlaskConical,
  Folder,
  FolderOpen,
  Home,
  Info,
  Keyboard,
  Lightbulb,
  ListChecks,
  LoaderCircle,
  MessageCircle,
  Mic,
  Monitor,
  Moon,
  NotebookPen,
  NotebookText,
  Palette,
  PanelLeft,
  PanelRight,
  Pause,
  Pencil,
  Play,
  Plus,
  RotateCcw,
  ScrollText,
  Search,
  Settings,
  Sparkles,
  Square,
  SquarePen,
  Sun,
  Tag,
  Trash2,
  TriangleAlert,
  Upload,
  Users,
  Workflow,
  Wrench,
  X,
} from 'lucide-react';

/** The type of every icon above, for registries such as the nav. */
export type Icon = LucideIcon;

/** Props that give every icon the house size and stroke. */
export const ICON_PROPS = { size: 16, strokeWidth: 1.5, 'aria-hidden': true } as const;

/** The larger size used only inside empty-state tiles (ui.md §3 Empty state). */
export const ICON_PROPS_LARGE = { size: 20, strokeWidth: 1.5, 'aria-hidden': true } as const;
