/** How long every mock command takes, so loading states show as they would. */
const MOCK_LATENCY_MS = 120;

/** Resolves after one mock round trip. */
export function latency(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, MOCK_LATENCY_MS);
  });
}
