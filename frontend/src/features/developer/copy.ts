/** Every string Settings → Developer shows, in one place (§3A). */
export const copy = {
  title: 'Sample meetings',
  description:
    'Loads eight realistic sample meetings with transcripts into this database. Loading them again restores them; your own meetings are left alone.',
  seed: 'Load sample meetings',
  seeding: 'Loading…',
  seeded: (meetings: number, segments: number) =>
    `Loaded ${meetings} sample meetings with ${segments.toLocaleString('en-US')} transcript segments.`,
} as const;

/** The command-line tool and Claude setup (DEV-02/04). */
export const cliCopy = {
  title: 'Command-line tool',
  description:
    'The deino command reads your meetings, notes and transcripts from Terminal, and serves them to Claude. It only ever reads.',
  bundled: 'Bundled with Deino',
  notBundled: 'This build does not include the command-line tool.',
  install: 'Install',
  reinstall: 'Reinstall',
  installing: 'Installing…',
  states: {
    missing: 'Not installed',
    installed: 'Installed',
    other_link: 'Points to another copy of Deino',
    other_file: 'Taken by another file',
  },
  dirs: {
    user_bin: { label: '~/.local/bin', hint: 'Just for you. Make sure it is on your PATH.' },
    usr_local_bin: { label: '/usr/local/bin', hint: 'On every PATH; may need an administrator.' },
  },
  needsAdmin: 'Needs an administrator',
  confirmTitle: 'Install the deino command?',
  confirmBody: (path: string) =>
    `Deino will create a link at ${path} that points to the command-line tool inside the app.`,
  cancel: 'Cancel',
  installed: (path: string) => `Installed at ${path}`,
  claudeTitle: 'Claude',
  claudeDescription:
    'Give Claude Code or Claude Desktop read access to your meetings through the deino MCP server.',
  claudeCode: 'Claude Code',
  claudeCodeHint: 'Run in Terminal. Add --scope user after "add" to use it in every project.',
  claudeDesktop: 'Claude Desktop',
  claudeDesktopHint: 'Add to ~/Library/Application Support/Claude/claude_desktop_config.json.',
  copy: (what: string) => `Copy ${what}`,
  copied: 'Copied',
  loadFailed: 'Could not read the command-line tool’s status.',
} as const;
