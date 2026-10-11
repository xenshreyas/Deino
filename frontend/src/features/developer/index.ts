/**
 * Settings → Developer: the `deino` command-line tool and its Claude setup
 * (the `developer` plugin, DEV-02/04), and in debug builds the sample
 * meetings (the `dev` plugin, loaded lazily so release builds drop it).
 */
export { developerApi } from './api';
export { DeveloperSettings } from './components/developer-settings';
export { cliStatusQuery, developerKeys, useCliStatus, useInstallCli } from './queries';
export type { CliStatus, InstallDir, InstallTarget, LinkState, SeedReport } from './types';
