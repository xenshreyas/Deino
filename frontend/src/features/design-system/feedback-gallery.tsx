/**
 * Feedback and overlays: alerts, skeletons, empty state, level bars, and
 * triggers for the popover, menu, dialog and toasts.
 */
import { toast } from 'sonner';

import { Alert, AlertDescription, AlertTitle } from '@/ui/alert';
import { Button } from '@/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/ui/dialog';
import { EmptyState } from '@/ui/empty-state';
import {
  CircleAlert,
  ICON_PROPS,
  Info,
  NotebookText,
  Pencil,
  Trash2,
  TriangleAlert,
} from '@/ui/icons';
import { LevelBars } from '@/ui/level-bars';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from '@/ui/menu';
import { Popover, PopoverContent, PopoverTrigger } from '@/ui/popover';
import { SkeletonText } from '@/ui/skeleton';

import { GallerySection } from './token-gallery';

/** Inline alerts in every tone. */
function Alerts() {
  return (
    <GallerySection title="Alerts">
      <Alert variant="info">
        <Info {...ICON_PROPS} />
        <AlertDescription>Transcription runs on this Mac.</AlertDescription>
      </Alert>
      <Alert variant="warning">
        <TriangleAlert {...ICON_PROPS} />
        <AlertTitle>No mic audio for 5s</AlertTitle>
        <AlertDescription>Check the input device.</AlertDescription>
      </Alert>
      <Alert variant="destructive">
        <CircleAlert {...ICON_PROPS} />
        <AlertDescription>Could not load your meetings.</AlertDescription>
      </Alert>
    </GallerySection>
  );
}

/** A popover and a menu. */
function FloatingDemos() {
  return (
    <>
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="secondary">Popover</Button>
        </PopoverTrigger>
        <PopoverContent>
          <p className="text-sm text-fg-muted">A raised surface with the popover shadow.</p>
        </PopoverContent>
      </Popover>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="secondary">Menu</Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          <DropdownMenuItem>
            <Pencil {...ICON_PROPS} />
            Rename<DropdownMenuShortcut>⌘R</DropdownMenuShortcut>
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem destructive>
            <Trash2 {...ICON_PROPS} />
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </>
  );
}

/** A confirmation dialog and two toasts. */
function ModalDemos() {
  return (
    <>
      <Dialog>
        <DialogTrigger asChild>
          <Button variant="secondary">Dialog</Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete this meeting?</DialogTitle>
            <DialogDescription>This cannot be undone.</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="ghost">Cancel</Button>
            <Button variant="danger-solid">Delete</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Button variant="secondary" onClick={() => toast.success('Meeting saved')}>
        Toast
      </Button>
      <Button
        variant="secondary"
        onClick={() => toast.error('Could not save', { description: 'Disk is full.' })}
      >
        Error toast
      </Button>
    </>
  );
}

/** Popover, menu, dialog and toast triggers (overlays render in the app theme). */
function Overlays() {
  return (
    <GallerySection title="Overlays">
      <div className="flex flex-wrap gap-2">
        <FloatingDemos />
        <ModalDemos />
      </div>
    </GallerySection>
  );
}

/** Loading, empty and live states. */
function States() {
  return (
    <GallerySection title="Loading, empty and live">
      <SkeletonText lines={3} />
      <div className="flex items-center gap-4">
        <LevelBars state="live" />
        <LevelBars state="paused" />
        <LevelBars state="idle" />
      </div>
      <EmptyState
        className="pt-4"
        icon={NotebookText}
        title="No meetings yet"
        description="Record your first meeting and it will appear here."
        action={<Button variant="secondary">Start recording</Button>}
      />
    </GallerySection>
  );
}

/** All feedback primitives. */
export function FeedbackGallery() {
  return (
    <div className="flex flex-col gap-6">
      <Alerts />
      <Overlays />
      <States />
    </div>
  );
}
