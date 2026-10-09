/**
 * The tags feature (ORG-06): tag CRUD through the `tags` plugin, the Tags
 * page, the meeting header's tag pills and combobox, the Meetings page's
 * tag filter and the ⌘K tag results. Other code imports from here only.
 */
export { tagsApi } from './api';
export { MeetingTags } from './components/meeting-tags';
export { TagFilter, type TagFilterProps } from './components/tag-filter';
export { TagDot, TagMark, TagPill } from './components/tag-pill';
export { TagPicker, type TagPickerProps } from './components/tag-picker';
export { TagsPalette } from './components/tags-palette';
export { TagsScreen } from './components/tags-screen';
export { tagsCopy } from './copy';
export { PALETTE, paletteSlot } from './palette';
export {
  tagKeys,
  tagListQuery,
  useApplyTag,
  useMeetingTags,
  useRemoveTag,
  useTagLinks,
  useTags,
} from './queries';
export {
  type AppliedTag,
  asTagId,
  type MeetingTagLink,
  type PaletteColor,
  type Tag,
  type TagId,
  type TagSource,
} from './types';
