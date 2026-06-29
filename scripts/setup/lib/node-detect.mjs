// How is Node installed, and what's the single safest command to get to >=22?
//
// Why this exists: the node probe (env-probe.mjs) only reports present/version —
// node carries `install.manager: "manual"` because we can't safely batch a node
// upgrade into the brew/npm one-liner (it's managed per-environment: nvm/brew/
// system). Without this, the setup *command* would have to hand-assemble a scary
// multi-command shell sniff (`which node; echo $NVM_DIR; brew list …`) in the
// consent path — exactly the "can't statically analyze this command" wall the
// working agreement forbids. So the determinism boundary does the detection and
// hands back ONE recommended command the command can show in an AskUserQuestion.
//
// Pure + seam-injected (lookupPath / env / home / fs) so tests never touch the real
// PATH, $HOME, or a real binary. Read-only: it scans PATH and stats files, nothing
// more.

import fsDefault from 'node:fs';
import pathMod from 'node:path';
import { pathLookup } from './env-probe.mjs';

// Resolve symlinks so a brew shim (/opt/homebrew/bin/node → …/Cellar/node/…) is
// recognized as brew even though the PATH entry doesn't say "Cellar". Best-effort:
// fall back to the PATH entry if realpath fails (dangling link / race).
function realpathSafe(p, fsmod) {
  try { return fsmod.realpathSync(p); } catch { return p; }
}

// Classify the node on PATH by where it lives. nvm and brew have telltale paths;
// /usr/bin and a bare /usr/local/bin (no Cellar behind it) are a system/manual
// install; anything else is unknown (volta, fnm, asdf, a tarball in $HOME, …).
function classifySource(execPath, realPath) {
  const both = `${execPath}\n${realPath}`;
  if (/\/\.nvm\//.test(both) || /\/\.fnm\//.test(both) || /\/\.volta\//.test(both)) {
    if (/\/\.nvm\//.test(both)) return 'nvm';
    return 'unknown'; // fnm/volta: detected as not-brew/not-nvm so we don't mis-advise
  }
  if (/\/Cellar\//.test(realPath) || /\/homebrew\//.test(execPath) || /\/\.linuxbrew\//.test(both)) {
    return 'brew';
  }
  if (/^\/usr\/(local\/)?bin\//.test(execPath) || /^\/usr\/bin\//.test(realPath)) return 'system';
  return 'unknown';
}

// Is nvm installed (even if not sourced into this non-interactive shell)? Presence
// of nvm.sh under $NVM_DIR or the default ~/.nvm is enough — that's what the
// recommended command will `source`.
function nvmDir({ env, home, fsmod }) {
  const dir = env.NVM_DIR || pathMod.join(home, '.nvm');
  return fsmod.existsSync(pathMod.join(dir, 'nvm.sh')) ? dir : null;
}

// Build the ONE command the command proposes to get Node >=minMajor, matched to how
// node is already managed so we never fight the user's setup:
//   - nvm present → source it and install/alias (nvm is a shell function, so the
//     command must source nvm.sh first; this is the single approved install, not a
//     sniff).
//   - brew present → install the pinned major formula and link it.
//   - neither → bootstrap nvm (least invasive, no sudo, doesn't touch system node),
//     then install.
// Returns { recommendedManager, recommendedCommand, note }.
function recommend({ source, nvmAvailable, brewAvailable, minMajor }) {
  if (source === 'nvm' || nvmAvailable) {
    const dir = '${NVM_DIR:-$HOME/.nvm}';
    return {
      recommendedManager: 'nvm',
      recommendedCommand: `export NVM_DIR="${dir}"; . "$NVM_DIR/nvm.sh" && nvm install ${minMajor} && nvm alias default ${minMajor}`,
      note: `Installs Node ${minMajor} via nvm and makes it your default.`,
    };
  }
  if (source === 'brew' || brewAvailable) {
    return {
      recommendedManager: 'brew',
      recommendedCommand: `brew install node@${minMajor} && brew link --overwrite --force node@${minMajor}`,
      note: `Installs Node ${minMajor} via Homebrew and links it onto your PATH.`,
    };
  }
  return {
    recommendedManager: 'nvm-bootstrap',
    recommendedCommand:
      `curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash && ` +
      `export NVM_DIR="$HOME/.nvm" && . "$NVM_DIR/nvm.sh" && nvm install ${minMajor} && nvm alias default ${minMajor}`,
    note: `No nvm or Homebrew found — installs nvm (no sudo, doesn't touch system Node), then Node ${minMajor}.`,
  };
}

// Inspect node and return everything the command needs to recommend an upgrade
// without improvising shell. `nodeResult` is the node entry from probeEnv().results.
//   { present, belowMin, absent, version, source, path,
//     nvmAvailable, brewAvailable, recommendedManager, recommendedCommand, note }
// When node is already present and new enough, recommended* are null (nothing to do).
export function detectNode(nodeResult, {
  lookupPath = pathLookup,
  env = process.env,
  home = process.env.HOME || '',
  fsmod = fsDefault,
  haveBrew = null, // pass the probe's brew result to avoid a second PATH scan
  minMajor = 22,
} = {}) {
  const version = nodeResult?.version ?? null;
  const present = Boolean(nodeResult?.present);
  // env-probe marks a too-old node present:false with a parsed version + below_min
  // reason; a missing binary has present:false and no version.
  const belowMin = !present && version != null;
  const absent = !present && version == null;

  const execPath = lookupPath('node');
  const realPath = execPath ? realpathSafe(execPath, fsmod) : null;
  const source = execPath ? classifySource(execPath, realPath) : 'unknown';

  const nDir = nvmDir({ env, home, fsmod });
  const nvmAvailable = nDir != null;
  const brewAvailable = haveBrew == null ? lookupPath('brew') != null : Boolean(haveBrew);

  const base = {
    present, belowMin, absent, version,
    source, path: execPath,
    nvmAvailable, brewAvailable,
    minMajor,
    recommendedManager: null, recommendedCommand: null, note: null,
  };
  if (present) return base; // new enough — nothing to recommend
  return { ...base, ...recommend({ source, nvmAvailable, brewAvailable, minMajor }) };
}
