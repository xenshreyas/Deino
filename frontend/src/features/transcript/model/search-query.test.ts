import { describe, expect, it } from 'vitest';

import type { SpeakerView } from '../types';
import {
  EMPTY_QUERY,
  isActive,
  nameMatches,
  parseText,
  promoteTokens,
  toggleSpeaker,
} from './search-query';

function view(key: string, name: string): SpeakerView {
  return { key, speakerId: null, name, isMe: false, color: 2, unnamed: false };
}

const SPEAKERS = [view('priya', 'Priya Shah'), view('paul', 'Paul Ito'), view('me', 'Me')];

describe('parseText', () => {
  it('splits words from bare and quoted speaker tokens, case-insensitively', () => {
    expect(parseText('budget Speaker:Priya "q4" speaker:"Paul Ito" plan')).toEqual({
      words: 'budget "q4" plan',
      names: ['priya', 'paul ito'],
    });
  });

  it('ignores an empty token and keeps a half-typed one as a name prefix', () => {
    expect(parseText('speaker:')).toEqual({ words: '', names: [] });
    expect(parseText('speaker:pr')).toEqual({ words: '', names: ['pr'] });
  });
});

describe('nameMatches', () => {
  it('matches the start of the name or of any word in it', () => {
    expect(nameMatches('Priya Shah', 'sha')).toBe(true);
    expect(nameMatches('Priya Shah', 'priya s')).toBe(true);
    expect(nameMatches('James', 'me')).toBe(false);
  });
});

describe('promoteTokens', () => {
  it('turns a complete token naming one speaker into a chip', () => {
    const query = promoteTokens({ text: 'speaker:priya budget', speakers: [] }, SPEAKERS);

    expect(query).toEqual({ text: 'budget', speakers: ['priya'] });
  });

  it('leaves a token that is still being typed or names several speakers', () => {
    const typing = promoteTokens({ text: 'speaker:pri', speakers: [] }, SPEAKERS);
    const ambiguous = promoteTokens({ text: 'speaker:p budget', speakers: [] }, SPEAKERS);

    expect(typing).toEqual({ text: 'speaker:pri', speakers: [] });
    expect(ambiguous).toEqual({ text: 'speaker:p budget', speakers: [] });
  });

  it('does not chip the same speaker twice', () => {
    const query = promoteTokens({ text: 'speaker:me ', speakers: ['me'] }, SPEAKERS);

    expect(query.speakers).toEqual(['me']);
  });
});

describe('toggleSpeaker and isActive', () => {
  it('adds and removes chips, and a query with only chips is a search', () => {
    const added = toggleSpeaker(EMPTY_QUERY, 'paul');

    expect(isActive(EMPTY_QUERY)).toBe(false);
    expect(isActive(added)).toBe(true);
    expect(toggleSpeaker(added, 'paul')).toEqual(EMPTY_QUERY);
    expect(isActive({ text: '   ', speakers: [] })).toBe(false);
  });
});
