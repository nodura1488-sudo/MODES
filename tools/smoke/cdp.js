'use strict';

async function connect(port = 9222) {
  const response = await fetch(`http://127.0.0.1:${port}/json`);
  const targets = await response.json();
  const target = targets.find((item) => item.type === 'page' && /TMStore/i.test(item.title)) || targets[0];
  if (!target) throw new Error('CDP target not found');

  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = () => reject(new Error('CDP websocket error'));
  });

  let nextId = 0;
  const pending = new Map();
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (!message.id || !pending.has(message.id)) return;
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) reject(new Error(JSON.stringify(message.error)));
    else resolve(message.result);
  };

  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++nextId;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  return { ws, send };
}

async function evaluate(cdp, expression) {
  const result = await cdp.send('Runtime.evaluate', {
    expression,
    returnByValue: true,
    awaitPromise: true
  });
  if (result.exceptionDetails) {
    throw new Error(JSON.stringify(result.exceptionDetails, null, 2));
  }
  return result.result.value;
}

async function screenshot(cdp, file) {
  const result = await cdp.send('Page.captureScreenshot', { format: 'png' });
  require('fs').writeFileSync(file, Buffer.from(result.data, 'base64'));
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

module.exports = { connect, evaluate, screenshot, sleep };
