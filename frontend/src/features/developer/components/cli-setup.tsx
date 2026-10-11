import type { UseMutationResult } from '@tanstack/react-query';

import type { IpcError } from '@/lib/ipc/result';
import { SettingsCard, SettingsSection } from '@/ui/settings-section';
import { Skeleton } from '@/ui/skeleton';

import { cliCopy } from '../copy';
import { useCliStatus, useInstallCli } from '../queries';
import type { CliStatus, InstallDir } from '../types';
import { InstallTargetRow } from './install-target-row';
import { Snippet } from './snippet';

/**
 * Settings → Developer's command-line setup: install the bundled `deino`
 * command, and copy the snippets that connect Claude Code and Claude
 * Desktop to its MCP server.
 */
export function CliSetup() {
  const status = useCliStatus();
  const install = useInstallCli();
  if (status.isPending) return <Skeleton className="h-40 w-full" />;
  if (status.isError) {
    return (
      <p role="alert" className="text-sm text-danger">
        {cliCopy.loadFailed}
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-8">
      <InstallSection status={status.data} install={install} />
      <ClaudeSection status={status.data} />
    </div>
  );
}

/** Props of {@link InstallSection}. */
interface InstallSectionProps {
  status: CliStatus;
  install: UseMutationResult<CliStatus, IpcError, InstallDir>;
}

/** Where the CLI is bundled, each folder it can be installed into, and the outcome. */
function InstallSection({ status, install }: InstallSectionProps) {
  return (
    <SettingsSection title={cliCopy.title} description={cliCopy.description}>
      {status.bundled === null ? (
        <p className="text-sm text-fg-muted">{cliCopy.notBundled}</p>
      ) : (
        <SettingsCard label={cliCopy.title}>
          <div className="px-4 py-3">
            <p className="text-sm text-fg">{cliCopy.bundled}</p>
            <p className="mt-0.5 font-mono text-xs break-all text-fg-muted">{status.bundled}</p>
          </div>
          {status.targets.map((target) => (
            <InstallTargetRow
              key={target.dir}
              target={target}
              busy={install.isPending}
              onInstall={() => {
                install.mutate(target.dir);
              }}
            />
          ))}
        </SettingsCard>
      )}
      <InstallOutcome install={install} />
    </SettingsSection>
  );
}

/** The last install's result: where it went, or why it failed. */
function InstallOutcome({ install }: Pick<InstallSectionProps, 'install'>) {
  if (install.isPending) return <p className="text-sm text-fg-muted">{cliCopy.installing}</p>;
  if (install.error) {
    return (
      <p role="alert" className="text-sm break-words text-danger">
        {install.error.message}
      </p>
    );
  }
  if (install.data?.command) {
    return (
      <p role="status" className="text-sm text-fg-muted">
        {cliCopy.installed(install.data.command)}
      </p>
    );
  }
  return null;
}

/** The Claude Code command and the Claude Desktop config, ready to copy. */
function ClaudeSection({ status }: { status: CliStatus }) {
  if (status.claudeCode === null || status.claudeDesktop === null) return null;
  return (
    <SettingsSection title={cliCopy.claudeTitle} description={cliCopy.claudeDescription}>
      <SettingsCard label={cliCopy.claudeTitle}>
        <Snippet
          label={cliCopy.claudeCode}
          hint={cliCopy.claudeCodeHint}
          text={status.claudeCode}
        />
        <Snippet
          label={cliCopy.claudeDesktop}
          hint={cliCopy.claudeDesktopHint}
          text={status.claudeDesktop}
        />
      </SettingsCard>
    </SettingsSection>
  );
}
