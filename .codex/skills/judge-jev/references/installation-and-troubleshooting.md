---
name: installation-and-troubleshooting
version: 0.1.0
description: Installing judge-jev into a repository or a parent folder with the judge-jev-installer skill (install.py), per-repository overrides and per-folder upgrades, the CLIs and credentials judge-jev depends on (python3, PyYAML, TypeSafe API key), minimum versions, PATH issues in Claude Code, Codex and Cursor environment policy, and known issues.
---

# Installation and troubleshooting

## Check first

```bash
which python3 && python3 --version
python3 -c 'import yaml; print("PyYAML", yaml.__version__)'
test -n "$TYPESAFE_API_KEY" && echo "environment key set (preferred)" || echo "environment key absent; check the documented platform credential store"
```

The environment-key message is informational: a documented local platform credential can also satisfy live Jev authentication.

## Install judge-jev into a folder

The marketplace plugin (`/plugin install sc-judge-jev`, the Codex and Cursor plugin catalogs) ships one skill, `judge-jev-installer`. It installs nothing globally: ask it to install judge-jev and it runs

```bash
uv run "<installer plugin>/install.py" --dest <folder> --allow-remote-judge|--no-remote-judge [--target claude,codex,cursor] [--dry-run]
```

`uv` reads the script's inline dependencies (PyYAML and the `sc-compose` wheel) and needs no pip step; from a judge-jev checkout, `uv run plugins/sc-judge-jev/install.py ...` is the same command. The installer writes `<folder>/.claude/skills/judge-jev/` (and `.codex/`, `.cursor/`: without `--target` every harness the folder uses, meaning the locked targets, existing installs and every harness directory present, `claude` when there is none; `--target` only adds one). It records the version, targets and the sha256 of every file it wrote in `<folder>/.sc/judge-jev/install-lock.json`; commit that file with the installed files. When no settings file says so yet, the installer skill asks once whether record text may be sent to TypeSafe (Jev); it never infers that consent from earlier runs.

**Destination.** `<folder>` is a repository, or a parent folder that holds several repositories (it need not be a git repository). Global locations are refused with `INSTALL.DEST_GLOBAL` (exit 2): the home directory itself, `~/.claude`, `~/.codex`, `~/.cursor`, `~/.agents`, `/etc/codex`, and any plugin cache under them.

**Parent-folder installs and overrides.** The trusted `jev_judge.py` is the one in the nearest `<dir>/.claude|.codex|.cursor/skills/judge-jev/scripts/`, walking up from the working directory and stopping before the home directory, and only when `<dir>/.sc/judge-jev/install-lock.json` lists that script: a copy the installer did not write (hand-copied, or with the lock removed), a copy under `.agents/`, or a copy in a global location fails with `VALIDATION.PATH`; rerun the installer to fix it. Settings follow the same walk, starting at the working directory even inside a repository: the nearest `.sc/judge-jev/settings.yaml` wins whole (files are not merged), so a repository under a parent install can override `allow_remote_judge` with its own file. Whether a harness discovers a skill installed above the repository it was started in is the harness's own rule; Claude Code and Codex document skill discovery up to the repository root, so start the session in the folder that holds the install, or install into the repository.

**Upgrade.** An upgrade is a rerun of the same command for that folder; each install folder is upgraded on its own. `install.py --dest <folder> --status` reports, read-only, the installed and installer versions (for example `installed 0.1.0, installer 0.2.0`). A rerun replaces only files that are unchanged since the installer wrote them, removes files this version no longer ships (agents, scripts, references, tests left inside the skill directory) when unchanged, and refuses, naming the file, when one was modified; `--overwrite` moves such a file to `<folder>/.backup/<UTC time>/<path>` and installs over it. It never writes over a file it does not own. Every check runs before anything is written; `--dry-run` prints the plan (writes, replacements, removals, settings) and writes nothing. Output is the JSON envelope `{success, data, error}`; exit 0 when the command ran, 2 on any error. The installer plugin's SessionStart hook prints one line when the nearest install is older than the plugin, and stays silent otherwise.

**Settings file.** Synaptic Canvas packages keep settings at `<folder>/.sc/<package>/settings.yaml`: YAML, snake_case keys, committed to git, never secrets. The file in effect is the nearest one at or above the working directory, stopping before the home directory: the repository's own file, else a parent folder's (see Parent-folder installs). `judge_jev_settings.py set` changes the file in effect (or creates one at the repository root, `git rev-parse --show-toplevel`). For this package:

```yaml
# .sc/judge-jev/settings.yaml
# Repository settings for the sc-judge-jev plugin; commit this file, never put secrets in it.
schema_version: 1
# allow_remote_judge: may record text be sent to TypeSafe (Jev) for judging? Only the user decides it.
allow_remote_judge: true
```

`schema_version` must be `1`. Unknown or duplicate keys are rejected (`SETTINGS.UNKNOWN_KEY`, `SETTINGS.MALFORMED`) so a typo never falls back silently. `.gitignore` may ignore `.sc/sessions/` and `.sc/*/cache/`, never this file.

## Install the CLIs

**python3** (3.11 or newer; `jev_judge.py` also needs `pydantic` and `sc-observability`, and `judge_jev_settings.py` needs PyYAML)

- macOS: `brew install python` or the Xcode command line tools' `/usr/bin/python3`
- Linux: `apt install python3` / `dnf install python3`
- Windows: `winget install Python.Python.3.12`; use forward slashes in paths

**PyYAML** (5.1 or newer; read only by `judge_jev_settings.py` for `.sc/judge-jev/settings.yaml`)

- `python3 -m pip install pyyaml` (into the interpreter the skill runs). Without it `judge_jev_settings.py` returns `DEPENDENCY.PYYAML` and exits 2; it never falls back to defaults.

**pydantic** (2.7 or newer, below 3; the model of every JSON input of `jev_judge.py`)

- `uv run --script jev_judge.py ...` installs it from the script's inline metadata, or `python3 -m pip install 'pydantic>=2.7,<3'`. Without it `jev_judge.py` returns `DEPENDENCY.PYDANTIC` and exits 2.

**sc-observability** (1.5.0; the run log of `jev_judge.py`)

- `uv run --script jev_judge.py ...` installs it from the script's inline metadata, or `python3 -m pip install 'sc-observability==1.5.0'`. Without it `jev_judge.py` returns `DEPENDENCY.SC_OBSERVABILITY` and exits 2.

**TypeSafe API key**

The live judge looks for the key in this order: `TYPESAFE_API_KEY` in the process environment, then the platform credential store. An invalid environment value fails closed; it does not silently select another credential. The platform entry is local-interactive convenience only—cloud, CI, and remote agents should use their platform secret injection.

- macOS: Keychain generic-password item with service `plan-anchor` and account `typesafe-api-key`.
- Windows: Credential Manager generic credential with target `plan-anchor:typesafe-api-key`.
- Linux: Secret Service item returned by `secret-tool lookup service plan-anchor account typesafe-api-key`. Create it without putting the key on a command line with `secret-tool store --label="plan-anchor TypeSafe API key" service plan-anchor account typesafe-api-key` and enter the value on standard input.

- Create a key at typesafe.ai. Read it silently so it does not appear in the command line, shell history, or terminal output, then export it in the shell that launches Claude Code or Codex:
  ```bash
  mkdir -p ~/.config/typesafe && chmod 700 ~/.config/typesafe
  printf 'TypeSafe API key: ' >&2
  IFS= read -r -s TYPESAFE_API_KEY
  printf '\n' >&2
  export TYPESAFE_API_KEY
  (umask 077; printf 'export TYPESAFE_API_KEY=%q\n' "$TYPESAFE_API_KEY" > ~/.config/typesafe/env)
  echo '[ -r "$HOME/.config/typesafe/env" ] && . "$HOME/.config/typesafe/env"' >> ~/.zshenv   # and ~/.bash_profile for bash
  ```
  GUI-launched apps do not read shell files; use `launchctl setenv TYPESAFE_API_KEY "$TYPESAFE_API_KEY"` (macOS, until reboot). Never commit or echo the key.
- Under Codex, `TYPESAFE_API_KEY` matches the default `*KEY*` exclusion for command environments. If you restrict `[shell_environment_policy]` in `config.toml`, keep `ignore_default_excludes = true`, and with an allowlist under `[shell_environment_policy.filters]` add `"TYPESAFE_API_KEY" = "include"`.
- Cursor's agent shell uses the environment of the Cursor app. A GUI launch does not read shell rc files, so set `TYPESAFE_API_KEY` with `launchctl setenv` (macOS, until reboot) before starting Cursor. A plugin variable does not export into `python3 jev_judge.py`.
- Live Jev calls also need the user's committed answer `allow_remote_judge: true` in `.sc/judge-jev/settings.yaml`. Record it once, at install time with `--allow-remote-judge` (or `--no-remote-judge`), or later with `python3 <skill>/scripts/judge_jev_settings.py set allow_remote_judge true|false`, and commit the file.

## Find an existing install

Claude Code's bash inherits a minimal PATH that can miss Homebrew, Go, pyenv, and `~/.local/bin`:

```bash
for p in /opt/homebrew/bin/python3 /usr/local/bin/python3 /usr/bin/python3 "$HOME/.pyenv/shims/python3"; do [ -x "$p" ] && echo "python3 at $p"; done
```

Use the full path, or `export PATH="/opt/homebrew/bin:$PATH"` for the session.

## Minimum versions

| Tool | Minimum | Why |
| --- | --- | --- |
| python3 | 3.11 | tested floor; standard library only, except `judge_jev_settings.py` |
| pydantic | 2.7 (below 3) | the strict model of every JSON input of `jev_judge.py` (ADR-JEV-0005) |
| sc-observability | 1.5.0 | the run log of `jev_judge.py` (REQ-JEV-0008) |
| PyYAML | 5.1 | `SafeLoader`-based strict reading of `.sc/judge-jev/settings.yaml`; CI tests 5.1 and the latest release |
| Jev | `jev-latest` | pin a version once thresholds are tuned |

## PATH troubleshooting

- A tool works in your terminal but not in the skill: Claude Code's bash did not source the shell init that added it to PATH. Export the directory in the session or use the full path.
- The key works in your terminal but the skill says it is missing: it was exported after Claude Code started, or only in an interactive rc file. Put it in `~/.zshenv`; restart Claude Code.

## Known issues

- `POLICY.REMOTE_JUDGE_DISABLED` on a live run: `allow_remote_judge` is `false` or absent in `.sc/judge-jev/settings.yaml`. If absent, ask the user once, record the answer with `judge_jev_settings.py set allow_remote_judge true|false`, and commit it. Never infer consent from earlier runs.
- `AUTH.CREDENTIALS` with the key set: it contains whitespace or a control character (usually a pasted trailing newline), or TypeSafe rejected it.
- `SETTINGS.UNKNOWN_KEY`, `SETTINGS.SCHEMA_VERSION`, `SETTINGS.MALFORMED`: fix `.sc/judge-jev/settings.yaml` to the form above.
- Installer `INSTALL.DEST_GLOBAL`: `--dest` named the home directory, a global harness directory or a plugin cache; pass a repository or a parent folder of repositories.
- `VALIDATION.PATH` with judge-jev installed: the script passed is not the nearest per-folder copy (or is a global one); pass the `jev_judge.py` of the install at or above the repository.
- Installer `INSTALL.CONFLICT`: `data.problems` names each file that was modified since it was installed or is not the installer's; restore or move it, or rerun with `--overwrite`.
