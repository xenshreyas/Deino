/**
 * Typed events for mock modules, shaped like tauri-specta's generated
 * `events.<name>` objects so a mock module's `events` can `satisfies` the
 * generated type.
 *
 * They ride on `@tauri-apps/api/event`, which `installMockIpc` points at
 * Tauri's in-page event mock: `emit` here reaches every `listen` in the app,
 * including `useIpcEvent` and the `DbChanged` bridge.
 */
import { emit, type EventCallback, listen, once, type UnlistenFn } from '@tauri-apps/api/event';

/** One event's `listen`/`once`/`emit`, as the generated bindings expose it. */
interface MockEventHandle<T> {
  listen: (handler: EventCallback<T>) => Promise<UnlistenFn>;
  once: (handler: EventCallback<T>) => Promise<UnlistenFn>;
  emit: (payload: T) => Promise<void>;
}

/** A generated event: usable directly or scoped to a window (mocks ignore the window). */
export type MockEvent<T> = MockEventHandle<T> & (() => MockEventHandle<T>);

/** A mock of the generated event named `name`, for example `plugin:db:db-changed`. */
export function mockEvent<T>(name: string): MockEvent<T> {
  const handle: MockEventHandle<T> = {
    listen: (handler) => listen<T>(name, handler),
    once: (handler) => once<T>(name, handler),
    emit: (payload) => emit(name, payload),
  };
  return Object.assign(() => handle, handle);
}
