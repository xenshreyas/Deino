/**
 * Global search (ORG-08) and the semantic index's settings (AI-01): the ⌘K
 * search and `@` filter providers, the `/search` results page, and
 * Settings → Search index. Other code imports from here only; routes read
 * the URL contract from `./search`.
 */
export { searchApi } from './api';
export { FilterChips, type FilterChipsProps } from './components/filter-chips';
export { HighlightedText, snippetParts } from './components/highlighted-text';
export { IndexSettings } from './components/index-settings';
export { FilterPalette } from './components/palette-filters';
export { SearchPalette, type SearchPaletteProps } from './components/palette-results';
export { SearchScreen, type SearchScreenProps } from './components/search-screen';
export { searchCopy } from './copy';
export { activeMention, momentTime } from './format';
export { hitTarget } from './hit-target';
export { searchKeys, searchQuery, useIndexStatus, useSearch } from './queries';
export type {
  FilterChip,
  FilterKind,
  Hit,
  HitKind,
  IndexStatus,
  SearchRequest,
  SearchResults,
  Snippet,
} from './types';
