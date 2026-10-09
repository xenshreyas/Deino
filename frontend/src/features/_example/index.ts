/**
 * The reference feature (dev-only): copy this folder to start a new one.
 *
 * | File | Role |
 * |---|---|
 * | `types.ts` | Types, re-exported from the generated bindings |
 * | `api.ts` | The only IPC calls: generated commands, `unwrap`ped |
 * | `queries.ts` | Query keys and TanStack Query hooks with `meta.tables` |
 * | `hooks/` | Other data hooks, such as typed event subscriptions |
 * | `components/` | A container plus presentational components |
 * | `copy.ts` | Every visible string |
 * | `*.test.tsx` | Colocated tests against typed binding mocks |
 *
 * Other code imports a feature through its `index.ts` only. Nothing in the
 * app imports this one, so release builds never include it.
 */
export { exampleApi } from './api';
export { ExampleNotes } from './components/example-notes';
export { exampleKeys, exampleNotesQuery, useAddExampleNote, useExampleNotes } from './queries';
export type { ExampleNote, ExampleNoteId } from './types';
