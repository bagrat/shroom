#!/usr/bin/env node
// setup — the deterministic backend for `/shroom:setup` (SPEC §8).
//
// The judgment (what to ask, when to install, how to phrase the gates) lives in
// the setup *command*; this script is the exact, repeatable mechanism it calls.
// It never mutates the machine on its own — `probe` only reads, and prints the
// install plan as a *proposal* the command surfaces for one approval.
//
// Subcommands:
//   probe [--json]   Check the local env (git/ffmpeg/wrangler/whisper) and print
//                    a consolidated install plan for whatever's missing.
// More subcommands (CF provisioning) land in M5b-2.

import { probeEnv, spawnRun } from './lib/env-probe.mjs';
import { buildInstallPlan } from './lib/install-plan.mjs';

async function haveBrew(run = spawnRun) {
  const res = await run('brew', ['--version']);
  return res.code === 0;
}

async function cmdProbe({ json }) {
  const env = await probeEnv();
  const plan = buildInstallPlan(env.results, { haveBrew: await haveBrew() });
  const out = { ...env, plan };

  if (json) {
    process.stdout.write(JSON.stringify(out, null, 2) + '\n');
    return env.ready ? 0 : 1;
  }

  for (const r of env.results) {
    const mark = r.present ? '✓' : r.required ? '✗' : '○';
    const ver = r.version ? ` ${r.version}` : '';
    const note = r.present ? '' : r.required ? '  (required)' : '  (optional)';
    process.stdout.write(`  ${mark} ${r.name}${ver}${note}\n`);
  }
  if (plan.nothingToInstall) {
    process.stdout.write('\nAll tools present.\n');
  } else {
    process.stdout.write('\nProposed install:\n');
    for (const step of plan.steps) {
      process.stdout.write(`  # ${step.label}${step.tools.length ? ` — ${step.tools.join(', ')}` : ''}\n`);
      process.stdout.write(`  ${step.command}\n`);
    }
  }
  return env.ready ? 0 : 1;
}

const [sub, ...rest] = process.argv.slice(2);
const json = rest.includes('--json');

let code = 0;
switch (sub) {
  case 'probe':
    code = await cmdProbe({ json });
    break;
  default:
    process.stderr.write('Usage: setup.mjs probe [--json]\n');
    code = 2;
}
process.exit(code);
