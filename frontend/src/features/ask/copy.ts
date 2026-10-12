/** User-visible strings of "Ask about this meeting". */
export const askCopy = {
  title: 'Ask about this meeting',
  emptyBody:
    'Answers come from this meeting’s notes and transcript, with the moments they rest on.',
  placeholder: 'Ask a question, or type / for recipes',
  send: 'Send',
  clear: 'Clear conversation',
  copyAnswer: 'Copy answer',
  copied: 'Answer copied.',
  retry: 'Try again',
  thinking: 'Reading the meeting…',
  sources: (n: number) => `Source ${n}`,
  removeRecipe: 'Remove recipe',
  recipesLabel: 'Recipes',
  noRecipes: 'No recipe matches.',
  threadLabel: 'Conversation about this meeting',
  suggestions: {
    decided: 'What was decided?',
  },
  suggestedRecipes: ['what-did-i-commit-to', 'write-follow-up-email'],
  preview: {
    title: 'Preview',
    body: 'Run this recipe on a meeting. Nothing is saved.',
    meeting: 'Meeting',
    choose: 'Choose a meeting',
    run: 'Run',
    running: 'Running…',
  },
} as const;
