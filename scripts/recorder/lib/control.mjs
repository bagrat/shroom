// Control fifo reader — the "fifo in" half of the recorder's two interfaces.
//
// The shim (or any process: `echo stop > control.fifo`) writes newline-delimited
// commands. We open the fifo with 'r+' (read-write) so the read stream never hits
// EOF when a writer disconnects — otherwise each `echo > fifo` would close our end.

import fs from 'node:fs';
import { EventEmitter } from 'node:events';

export function watchControl(fifoPath) {
  const emitter = new EventEmitter();
  const stream = fs.createReadStream(fifoPath, { flags: 'r+' });
  let buf = '';

  stream.on('data', (chunk) => {
    buf += chunk.toString('utf8');
    let idx;
    while ((idx = buf.indexOf('\n')) >= 0) {
      const line = buf.slice(0, idx).trim();
      buf = buf.slice(idx + 1);
      if (line) emitter.emit('command', line);
    }
  });
  stream.on('error', (e) => emitter.emit('error', e));

  emitter.close = () => {
    try { stream.close(); } catch { /* already closed */ }
  };
  return emitter;
}
