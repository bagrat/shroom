// Control fifo reader — the "fifo in" half of the recorder's two interfaces.
//
// The shim (or any process: `echo stop > control.fifo`) writes newline-delimited
// commands. Two open flags matter, both load-bearing:
//   O_RDWR     — hold the fifo open read-write so the reader never sees EOF when a
//                writer disconnects (otherwise each `echo > fifo` would end our
//                read). The process is its own writer, so the pipe never empties.
//   O_NONBLOCK — CRITICAL for clean shutdown. fs.createReadStream does *blocking*
//                reads on the libuv threadpool; a blocking read parked on a fifo
//                wedges process.exit() forever — the recorder finalized on /stop
//                but never exited, so its harness-tracked task never completed and
//                the publish flow stalled. A non-blocking fd driven by net.Socket
//                polls the event loop and tears down cleanly, so exit works.

import fs from 'node:fs';
import net from 'node:net';
import { EventEmitter } from 'node:events';

export function watchControl(fifoPath) {
  const emitter = new EventEmitter();
  const fd = fs.openSync(fifoPath, fs.constants.O_RDWR | fs.constants.O_NONBLOCK);
  const stream = new net.Socket({ fd, readable: true, writable: false });
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
    try { stream.destroy(); } catch { /* already closed */ }
  };
  return emitter;
}
