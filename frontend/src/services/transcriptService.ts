/**
 * Transcript search through the legacy `api_search_transcripts` command,
 * for the ⌘K palette until ORG-08's full-text search replaces it. The live
 * transcript is the `recording` feature's (CAP-02).
 */

import { invoke } from '@tauri-apps/api/core';

/** One meeting whose transcript matches a search (legacy `api_search_transcripts`). */
export interface TranscriptSearchResult {
  /** The meeting ID. */
  id: string;
  title: string;
  /** Transcript text around the match. */
  matchContext: string;
  timestamp: string;
}

export class TranscriptService {
  /**
   * Meetings whose transcript contains `query`, for the ⌘K palette.
   * Replaced by ORG-08's full-text search.
   */
  async searchTranscripts(query: string): Promise<TranscriptSearchResult[]> {
    return invoke<TranscriptSearchResult[]>('api_search_transcripts', { query });
  }
}

export const transcriptService = new TranscriptService();
