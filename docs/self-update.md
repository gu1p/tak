# Self-Update

`tak` and `takd` can update themselves from signed GitHub releases. The goal is to
stop hand-updating a fleet of `takd` agents: each node checks for newer releases
and, when configured, swaps its own binary and restarts — keeping its identity,
config, and data.

## What an update touches

Only the binary files (`tak`/`takd`) are replaced. **Preserved across updates:**

- config (`agent.toml` in the config root),
- state (the SQLite stores, bearer token, Tor onion keys in the state root),
- the client inventory (`remotes.toml`).

A node keeps its node id, token, and onion address; it just runs a newer version.

## Trust model

Authenticity comes from a **minisign (Ed25519) signature** verified against a
public key embedded in the binary (`crates/tak-update/keys/release.pub`). A
release archive is only installed if:

1. its `.minisig` signature verifies against the embedded key (authenticity), then
2. its `.sha256` matches the downloaded bytes (integrity), then
3. the extracted binary's `--version` exactly equals the release tag (sanity).

TLS to `github.com` protects the download in transit. The signature is the
boundary that defends against a compromised GitHub account/CDN: without the
private signing key, a forged archive cannot be installed.

## Manual updates

```
tak update            # update tak (and a co-located takd) to the latest release
tak update --check    # report whether a newer version exists; install nothing
tak update --version 0.1.40   # install a specific tag
tak update --force    # allow downgrade / override the system-path guard
```

`takd update [...]` is the same for the daemon binary. It is a separate
short-lived process: it swaps the on-disk binaries and prints the command to
restart the running service (`systemctl --user restart takd.service`).

## Unattended daemon auto-update

The `takd serve` daemon runs a background loop that periodically checks for a
newer signed release and, when configured to apply, drains in-flight work, swaps
the binary, and exits so the supervisor (systemd `Restart=always` / launchd
`KeepAlive`) restarts into the new version.

Configure it under `[auto_update]` in `agent.toml`:

| field | default | meaning |
|---|---|---|
| `enabled` | `true` | master switch / kill switch for the loop |
| `auto_apply` | `true` | swap automatically (vs check-and-log only) |
| `network` | `clearnet` on `direct`, `disabled` on `tor` | where fetches may go (`disabled`/`clearnet`) |
| `require_signature` | `true` | reserved authenticity gate (signature is always required) |
| `check_interval_hours` | `24` | base check cadence |
| `jitter_hours` | `6` | random 0..N hours added per check (anti fleet-storm) |
| `repo` | built-in | release repository override (`owner/name`) |
| `pinned_version` | unset | pin a tag instead of tracking latest |
| `allow_downgrade` | `false` | permit installing an older version |
| `include_sibling_tak` | `true` | also replace a co-located `tak` binary |
| `drain_timeout_secs` | `1800` | max wait for in-flight tasks before applying |

Example (`agent.toml`):

```toml
[auto_update]
enabled = true
auto_apply = true
check_interval_hours = 12
```

### Kill switches

- Set `enabled = false` (or `auto_apply = false` to keep checking but not apply).
- Set the environment variable `TAKD_NO_AUTO_UPDATE=1` for the service.

### Tor nodes

A `transport = "tor"` node defaults to `network = "disabled"`: it will **not**
reach out to the clearnet release host, preserving its anonymity. To opt a tor
node into clearnet auto-update, set `network = "clearnet"` explicitly. (Fetching
the release over Tor is a future enhancement.)

## Safety

- **Validate before swap** — the downloaded binary is executed with `--version`
  and must equal the release tag before any live binary is touched.
- **Atomic swap** — new bytes are staged in the target's own directory, fsynced,
  and `rename(2)`'d over the live path; a failure leaves the original intact.
- **All-or-nothing** — `tak` and `takd` are validated together, then swapped with
  rollback of the already-swapped binary if the second fails.
- **Drain** — in-flight remote tasks are allowed to finish (up to
  `drain_timeout_secs`) before the daemon restarts.
- **Crash-loop rollback** — after an update the daemon keeps `.bak` copies; if the
  new binary fails to stay up across several restarts, the previous binary is
  restored automatically and the node stops updating until intervened.
- **Install-location guards** — self-update refuses package-manager paths
  (`/usr`, `/opt`, `/nix/store`, Homebrew, …) and read-only directories, so it
  never fights `apt`/`brew`/Nix; the standard `~/.local/bin` install is updatable.

## Maintainer setup: release signing

Releases are signed in CI. To enable signing:

1. Generate a keypair (passwordless, so CI can sign non-interactively):
   ```
   rsign generate -W -p crates/tak-update/keys/release.pub -s release.key
   ```
   (or `minisign -G`). Commit `crates/tak-update/keys/release.pub`.
2. Add the **secret** key file's contents as the GitHub Actions secret
   `TAK_MINISIGN_SECRET_KEY` (repo → Settings → Secrets and variables → Actions).
3. The release workflow signs each archive (`*.tar.gz.minisig`) and uploads it
   alongside the archive and its `.sha256`.

To **rotate** the key, repeat the steps with a new keypair and ship a release
built from the new public key before retiring the old one. If
`TAK_MINISIGN_SECRET_KEY` is unset, the release publish job **fails** (fail-closed)
rather than shipping an unsigned release — set the secret before merging to `main`.
