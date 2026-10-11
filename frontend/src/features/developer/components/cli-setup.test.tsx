import { QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type * as Generated from '@/lib/ipc/generated/developer';
import { createQueryClient } from '@/lib/ipc';
import { TooltipProvider } from '@/ui/tooltip';

import { cliCopy } from '../copy';
import type { CliStatus } from '../types';
import { CliSetup } from './cli-setup';

// Typed against the generated bindings: a Rust signature change breaks this mock.
const commands = vi.hoisted(
  () =>
    ({
      cliStatus: vi.fn<typeof Generated.commands.cliStatus>(),
      installCli: vi.fn<typeof Generated.commands.installCli>(),
    }) satisfies typeof Generated.commands,
);
vi.mock('@/lib/ipc/generated/developer', () => ({ commands }));

const BUNDLED = '/Applications/Deino.app/Contents/MacOS/cli';
const LINK = '/Users/me/.local/bin/deino';

function status(installed: boolean): CliStatus {
  const command = installed ? LINK : BUNDLED;
  return {
    bundled: BUNDLED,
    targets: [
      {
        dir: 'user_bin',
        path: LINK,
        state: { kind: installed ? 'installed' : 'missing' },
        writable: true,
      },
      {
        dir: 'usr_local_bin',
        path: '/usr/local/bin/deino',
        state: { kind: 'missing' },
        writable: false,
      },
    ],
    command,
    claudeCode: `claude mcp add deino -- ${command} mcp`,
    claudeDesktop: '{ "mcpServers": {} }',
  };
}

function renderSetup() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TooltipProvider>
        <CliSetup />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

/** Opens the confirmation for the first folder and confirms it. */
async function installIntoUserBin() {
  const [install] = await screen.findAllByRole('button', { name: cliCopy.install });
  if (install === undefined) throw new Error('no Install button');
  fireEvent.click(install);
  const dialog = await screen.findByRole('dialog');
  expect(dialog.textContent).toContain(LINK);
  fireEvent.click(within(dialog).getByRole('button', { name: cliCopy.install }));
}

describe('CliSetup', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    commands.cliStatus.mockResolvedValue({ status: 'ok', data: status(false) });
  });

  it('shows the bundled CLI and the Claude Code command', async () => {
    renderSetup();

    expect(await screen.findByText(BUNDLED)).toBeDefined();
    expect(screen.getByText(`claude mcp add deino -- ${BUNDLED} mcp`)).toBeDefined();
    expect(screen.getAllByText(cliCopy.states.missing, { exact: false })).toHaveLength(2);
  });

  it('installs only after the user confirms the exact link', async () => {
    commands.installCli.mockResolvedValue({ status: 'ok', data: status(true) });
    renderSetup();

    await installIntoUserBin();

    expect(commands.installCli).toHaveBeenCalledWith('user_bin');
    expect((await screen.findByRole('status')).textContent).toBe(cliCopy.installed(LINK));
    expect(screen.getByText(`claude mcp add deino -- ${LINK} mcp`)).toBeDefined();
  });

  it('shows why an install failed', async () => {
    commands.installCli.mockResolvedValue({
      status: 'error',
      error: { code: 'conflict', message: `${LINK} already exists` },
    });
    renderSetup();

    await installIntoUserBin();

    expect((await screen.findByRole('alert')).textContent).toBe(`${LINK} already exists`);
  });

  it('copies a snippet to the clipboard', async () => {
    const writeText = vi.fn<(text: string) => Promise<void>>().mockResolvedValue();
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    renderSetup();

    fireEvent.click(await screen.findByRole('button', { name: cliCopy.copy(cliCopy.claudeCode) }));

    expect(writeText).toHaveBeenCalledWith(`claude mcp add deino -- ${BUNDLED} mcp`);
  });
});
