// Turn probe results into a consolidated install plan (SPEC §8 step 2). The
// working agreement is "propose → confirm → run" as ONE approval, not N — so this
// collapses every missing tool into the fewest exact commands a human can okay in
// one go. It builds the commands; it never runs them (the command does that, once,
// after the single AskUserQuestion approval).

// How each package manager batches installs into a single command.
const MANAGERS = {
  brew: { label: 'Homebrew', command: (pkgs) => `brew install ${pkgs.join(' ')}` },
  npm: { label: 'npm (global)', command: (pkgs) => `npm install -g ${pkgs.join(' ')}` },
};

// Build the plan from probe results (see env-probe.probeEnv().results).
// `haveBrew`: whether `brew` itself is on PATH — if a brew-managed tool is missing
// and brew is absent, surface installing brew as a prerequisite rather than
// emitting a command that would just fail.
export function buildInstallPlan(results, { haveBrew = true } = {}) {
  const missing = results.filter((r) => !r.present);
  const required = missing.filter((r) => r.required);
  const optional = missing.filter((r) => !r.required);

  // Group missing tools by their package manager, preserving catalogue order.
  const byManager = new Map();
  for (const r of missing) {
    const mgr = r.install?.manager;
    if (!mgr || !MANAGERS[mgr]) continue;
    if (!byManager.has(mgr)) byManager.set(mgr, []);
    byManager.get(mgr).push(r);
  }

  const steps = [];
  const needsBrew = byManager.has('brew') && !haveBrew;
  if (needsBrew) {
    // brew is itself a system mutation; show its official one-liner so the whole
    // chain is one reviewable approval rather than a dead-end command.
    steps.push({
      manager: 'brew-bootstrap',
      label: 'Install Homebrew (prerequisite)',
      tools: [],
      command:
        '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"',
    });
  }
  for (const [mgr, tools] of byManager) {
    const pkgs = tools.map((t) => t.install.package);
    steps.push({
      manager: mgr,
      label: MANAGERS[mgr].label,
      tools: tools.map((t) => t.name),
      command: MANAGERS[mgr].command(pkgs),
    });
  }

  return {
    nothingToInstall: missing.length === 0,
    requiredMissing: required.map((r) => r.name),
    optionalMissing: optional.map((r) => r.name),
    needsBrew,
    steps,
    // The single string a human approves: every command, newline-joined.
    combinedCommand: steps.map((s) => s.command).join(' && '),
  };
}
