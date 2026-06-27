// events.ndjson writer — the "events out" half of the recorder's two interfaces.
//
// Append-only newline-delimited JSON. Each record is { ts, event, ...fields }.
// This file is also the durable recovery / pending-publish artifact (SPEC §6):
// it survives session death and is drained by the next /shroom run. We also echo
// each record to stdout so a live (harness-tracked) parent sees events in real time.

import fs from 'node:fs';

export function createEventLog(eventsPath) {
  const fd = fs.openSync(eventsPath, 'a');
  return {
    emit(event, fields = {}) {
      const rec = { ts: new Date().toISOString(), event, ...fields };
      const line = JSON.stringify(rec) + '\n';
      fs.writeSync(fd, line);
      process.stdout.write(line);
      return rec;
    },
    close() {
      try { fs.closeSync(fd); } catch { /* already closed */ }
    },
  };
}
