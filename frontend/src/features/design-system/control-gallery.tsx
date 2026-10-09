/**
 * The interactive primitives in their states: buttons, inputs, select,
 * switch, segmented control, tabs, chips and keys.
 */
import { useState } from 'react';

import { Button } from '@/ui/button';
import { Chip, MetaChip } from '@/ui/chip';
import { Label, Switch } from '@/ui/controls';
import { IconButton } from '@/ui/icon-button';
import { Calendar, ICON_PROPS, Monitor, Moon, Plus, Sparkles, SquarePen, Sun } from '@/ui/icons';
import { Input } from '@/ui/input';
import { Kbd } from '@/ui/kbd';
import { SegmentedControl } from '@/ui/segmented-control';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/ui/select';
import { Tabs, TabsList, TabsTrigger } from '@/ui/tabs';

import { GallerySection } from './token-gallery';

/** Every button variant and size. */
function Buttons() {
  return (
    <GallerySection title="Buttons">
      <div className="flex flex-wrap items-center gap-2">
        <Button>Primary</Button>
        <Button variant="secondary">Secondary</Button>
        <Button variant="ghost">Ghost</Button>
        <Button variant="accent">Accent</Button>
        <Button variant="danger">Danger</Button>
        <Button variant="link">Link</Button>
        <Button disabled>Disabled</Button>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="secondary">
          <Sparkles {...ICON_PROPS} />
          Small
        </Button>
        <Button size="md" variant="secondary">
          Medium
        </Button>
        <Button size="lg">Large</Button>
        <IconButton icon={SquarePen} label="New note" shortcut="⌘N" />
        <Kbd>⌘K</Kbd>
      </div>
    </GallerySection>
  );
}

/** Text field, select and switch. */
function Fields() {
  const [enabled, setEnabled] = useState(true);
  return (
    <GallerySection title="Inputs">
      <div className="grid grid-cols-2 gap-2">
        <Input placeholder="Placeholder" />
        <Input defaultValue="Invalid value" aria-invalid />
        <Select defaultValue="parakeet">
          <SelectTrigger aria-label="Engine">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="parakeet">Parakeet</SelectItem>
            <SelectItem value="whisper">Whisper</SelectItem>
          </SelectContent>
        </Select>
        <Input disabled placeholder="Disabled" />
      </div>
      <div className="flex items-center gap-2">
        <Switch id={`switch-${String(enabled)}`} checked={enabled} onCheckedChange={setEnabled} />
        <Label>Capture system audio</Label>
      </div>
    </GallerySection>
  );
}

/** Tabs, segmented control and chips. */
function Navigation() {
  const [theme, setTheme] = useState<'system' | 'light' | 'dark'>('system');
  return (
    <GallerySection title="Tabs, segments and chips">
      <Tabs defaultValue="notes">
        <TabsList>
          <TabsTrigger value="notes">Notes</TabsTrigger>
          <TabsTrigger value="transcript">Transcript</TabsTrigger>
          <TabsTrigger value="insights">Insights</TabsTrigger>
        </TabsList>
      </Tabs>
      <SegmentedControl
        label="Theme"
        value={theme}
        onValueChange={setTheme}
        segments={[
          { value: 'system', label: 'System', icon: Monitor },
          { value: 'light', label: 'Light', icon: Sun },
          { value: 'dark', label: 'Dark', icon: Moon },
        ]}
      />
      <div className="flex flex-wrap gap-1.5">
        <MetaChip icon={Calendar}>Today 14:00</MetaChip>
        <Chip icon={Plus} variant="ghost">
          Add tag
        </Chip>
        <Chip>Planning</Chip>
      </div>
    </GallerySection>
  );
}

/** All controls. */
export function ControlGallery() {
  return (
    <div className="flex flex-col gap-6">
      <Buttons />
      <Fields />
      <Navigation />
    </div>
  );
}
