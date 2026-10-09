# Self-hosted macOS runner (optional)

Deino's repository is private, and GitHub bills macOS runner minutes at ten
times the Linux rate: the free plan's ~2,000 minutes a month buy about 200
macOS minutes, less than one cold build of the app. So CI (`.github/workflows/ci.yml`)
runs on Linux for every push, and the macOS job only runs by hand.

Registering your own Mac as a runner makes macOS CI free and fast: jobs run on
Apple Silicon you already own, keep their `target/` between runs, and share
the sccache compile cache. With a runner registered and the `MACOS_RUNNER`
variable set (below):

- the **macOS job** of `ci.yml` also runs weekly (Mondays 06:00 UTC) with the
  full gate (`scripts/ci-local.sh`, every step), and manual runs use your Mac;
- **`release-macos.yml`** builds the DMG on your Mac.

Nothing else changes: pushes still run on GitHub's Linux runners. Without the
variable, both workflows fall back to GitHub's `macos-15` runners.

You never need this to work on Deino: `scripts/ci-local.sh` runs the same gate
locally before every merge.

## Before you start: security

A self-hosted runner executes whatever a workflow tells it to, as the user it
runs as, on your Mac. GitHub's guidance is to use them **only with private
repositories**, which Deino is. Even so:

- **Anyone with write access can run code on your Mac.** A pushed branch can
  change a workflow to target `self-hosted`. Today that is you and the agents
  you run on this same Mac; review who has write access before adding anyone.
- **Fork pull requests must not reach it.** In the repository's *Settings →
  Actions → General*, leave *Fork pull request workflows in private
  repositories* off (the default).
- **Jobs are not isolated.** Files, caches and processes persist between jobs.
  Run the runner as a **dedicated standard (non-admin) macOS user**, for
  example `deino-ci`, that has no personal keychain items, SSH keys, cloud
  credentials, browser sessions or iCloud account. Never run it as your own
  user or with `sudo`.
- **No secrets are needed.** Deino's workflows use only the automatic,
  read-only `GITHUB_TOKEN`; do not add repository secrets for the runner.
- **Keep it outside synced folders.** Put the runner (and so its work
  directory) in the CI user's home, not under `~/Documents` or `~/Desktop`:
  iCloud and File Provider attach extended attributes that break `codesign`.
- **Keep it updated.** The runner updates itself; leave that on.
- **Remove it when unused** (see the end of this page).

## 1. Create the CI user and its toolchain

1. *System Settings → Users & Groups → Add User*: a **Standard** account,
   for example `deino-ci`. Log in to it once (or use `su - deino-ci` in
   Terminal for the remaining steps).
2. Install the build prerequisites for that user (see the root README):
   ```bash
   xcode-select --install                      # Xcode Command Line Tools
   curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
   brew install cmake node@22 pnpm sccache     # Homebrew is shared; skip if present
   cargo install cargo-deny --locked
   cargo install typos-cli --locked
   cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features sqlite
   brew install cargo-nextest
   ```
   The workflow installs the pinned Rust toolchain itself (`rust-toolchain.toml`).
   `scripts/ci-local.sh` names any tool that is missing.

## 2. Register the runner

1. On GitHub: **xenshreyas/Deino → Settings → Actions → Runners → New
   self-hosted runner**, choose **macOS** and **ARM64**. Keep the page open:
   it shows the download commands and a registration token that expires
   after an hour.
2. As the CI user, run the download commands shown there into
   `~/actions-runner`, then configure it for this repository only:
   ```bash
   cd ~/actions-runner
   ./config.sh --url https://github.com/xenshreyas/Deino --token <TOKEN> \
     --name "$(scutil --get LocalHostName)" --work _work --unattended
   ```
   macOS runners get the labels `self-hosted`, `macOS` and `ARM64`
   automatically; add none of your own.
3. Give the runner the CI user's `PATH` (a launchd service does not read shell
   profiles). `config.sh` saved the current `PATH` in `~/actions-runner/.path`;
   check that it contains `~/.cargo/bin` and `/opt/homebrew/bin`, and edit it
   if not.
4. Install and start it as a launchd service, so it survives reboots:
   ```bash
   ./svc.sh install
   ./svc.sh start
   ./svc.sh status
   ```
   The runner now shows as **Idle** on the Runners page.

## 3. Point the workflows at it

The workflows choose their macOS runner from the repository variable
`MACOS_RUNNER`, a JSON label list:

```bash
gh variable set MACOS_RUNNER -R xenshreyas/Deino --body '["self-hosted", "macOS", "ARM64"]'
```

(or *Settings → Secrets and variables → Actions → Variables*). With the
variable set, the macOS jobs:

- run on your Mac and skip GitHub's build cache and tool downloads, so they
  use the CI user's tools, `target/` and sccache;
- run the weekly scheduled full gate (a scheduled run is skipped while the
  variable is unset, so it never spends hosted macOS minutes).

Try it: **Actions → CI → Run workflow**, choose `full`. The first run builds
from cold (about as long as a first local build); later runs are incremental.
Delete the variable to go back to GitHub-hosted runners.

## Maintenance

- **Disk:** the work directory (`~/actions-runner/_work`) holds a full
  `target/` (tens of GB) plus the sccache cache (`SCCACHE_CACHE_SIZE`, 80 GB by
  default in `scripts/dev-env.sh`). Clear `_work/Deino/Deino/target` if space
  runs low.
- **Logs:** `~/actions-runner/_diag/`.
- **Pause:** `./svc.sh stop`. Queued macOS jobs wait until it is back (or
  until you delete `MACOS_RUNNER`).

## Remove the runner

```bash
cd ~/actions-runner
./svc.sh stop && ./svc.sh uninstall
./config.sh remove --token <TOKEN>   # token from Settings → Actions → Runners → the runner → Remove
gh variable delete MACOS_RUNNER -R xenshreyas/Deino
```

Then delete the CI user if you no longer need it.
