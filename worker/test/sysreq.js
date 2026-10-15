// The system-requirements sentence ("For macOS 14 or newer, Apple silicon and Intel.") is
// ratified as a shape, not as words: "fill the facts in, don't reword" (SM:214, the retired
// spec text; the landing and join-landing notes in spec/machine.json). So the join page's line is checked
// against the facts that ship, exactly as site/test/landing.test.mjs checks the landing's:
// the installer's macOS floor, and the kinds of Mac the release packager lets through
// (its `case "$arch"` arms that don't exit). DECISIONS B7 keeps Intel Bagrat's call.
import { readFileSync } from "node:fs";

const read = (p) => readFileSync(new URL(`../../${p}`, import.meta.url), "utf8");

export const SYSREQ = /^For macOS (\d+) or newer, (.+)\.$/;

const floor = read("installer/install.sh").match(/^MIN_MACOS=(\d+)$/m)?.[1];
const archCase = read("tools/package-release.sh").match(/case "\$arch" in([\s\S]*?)esac/)?.[1] ?? "";
const shipped = [...archCase.matchAll(/^\s*([a-z0-9_|]+)\)\s*;;/gm)].flatMap((m) => m[1].split("|"));
const KIND = { arm64: "Apple silicon", x86_64: "Intel" };
const kinds = ["arm64", "x86_64"].filter((a) => shipped.includes(a)).map((a) => KIND[a]).join(" and ");

/** The ratified sentence with this build's facts filled in, or null if a fact can't be read. */
export const sysreqLine = floor && kinds ? `For macOS ${floor} or newer, ${kinds}.` : null;
