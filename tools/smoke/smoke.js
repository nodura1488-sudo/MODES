'use strict';

const fs = require('fs');
const path = require('path');
const { spawn, execFileSync } = require('child_process');
const { connect, evaluate, screenshot, sleep } = require('./cdp');

const ROOT = path.resolve(__dirname, '..', '..');
const args = process.argv.slice(2);

function argValue(name, fallback) {
  const index = args.indexOf(name);
  if (index === -1) return fallback;
  return args[index + 1] || fallback;
}

const exe = path.resolve(ROOT, argValue('--exe', 'TMStore.exe'));
const port = Number(argValue('--port', '9262')) || 9262;
const allowRunning = args.includes('--allow-running');
const artifacts = path.join(__dirname, 'artifacts');
fs.mkdirSync(artifacts, { recursive: true });

function fail(message) {
  console.error(`FAIL: ${message}`);
  process.exitCode = 1;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function launcherIsRunning() {
  try {
    const output = execFileSync('tasklist', ['/FI', 'IMAGENAME eq tmstore.exe'], { encoding: 'utf8' });
    return /tmstore\.exe/i.test(output);
  } catch {
    return false;
  }
}

function killTree(pid) {
  try {
    execFileSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
  } catch {
    // Process may already be gone.
  }
}

async function waitFor(check, timeoutMs, intervalMs = 150) {
  const started = Date.now();
  let lastError = null;
  while (Date.now() - started < timeoutMs) {
    try {
      const value = await check();
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await sleep(intervalMs);
  }
  throw new Error(`Timeout after ${timeoutMs} ms${lastError ? `: ${lastError.message}` : ''}`);
}

(async () => {
  let child = null;
  let cdp = null;

  try {
    assert(fs.existsSync(exe), `Launcher not found: ${exe}`);
    assert(!launcherIsRunning() || allowRunning, 'TMStore is already running. Close it before the smoke test.');

    child = spawn(exe, [], {
      cwd: path.dirname(exe),
      env: {
        ...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
        WEBVIEW2_USER_DATA_FOLDER: path.join(artifacts, 'webview2', String(port))
      },
      stdio: 'ignore',
      windowsHide: false
    });
    if (!child.pid) throw new Error('Failed to start launcher');

    cdp = await waitFor(async () => {
      try {
        return await connect(port);
      } catch {
        return null;
      }
    }, 25000);
    assert(cdp, 'CDP is not available');

    await cdp.send('Runtime.enable');

    await waitFor(async () => {
      try {
        return await evaluate(cdp, `typeof S !== 'undefined' && !S.loading && document.getElementById('splash')?.classList.contains('done')`);
      } catch {
        return false;
      }
    }, 45000);
    console.log('PASS: launcher finished startup');

    await waitFor(async () => {
      try {
        return (await evaluate(cdp, `typeof S !== 'undefined' && S.modes.length`)) > 0;
      } catch {
        return false;
      }
    }, 30000);

    const catalog = await evaluate(cdp, `(() => ({
      count: S.modes.length,
      ids: S.modes.map((mode) => mode.id),
      authors: S.modes.map((mode) => modeAuthors(mode))
    }))()`);
    assert(catalog.count > 0, 'catalog is empty');
    assert(catalog.authors.every((authors) => authors.length > 0), 'some modes have no author');
    console.log(`PASS: catalog loaded (${catalog.count} modes)`);

    const settings = await evaluate(cdp, `(async () => {
      openModal('modal-settings');
      await new Promise((resolve) => setTimeout(resolve, 500));
      const advanced = document.querySelector('.settings-advanced');
      return {
        hasCatalogInput: !!document.getElementById('set-catalog-url'),
        hasDataDirInput: !!document.getElementById('set-data-dir'),
        tabCount: document.querySelectorAll('.settings-tab').length,
        hasAdvanced: !!advanced,
        advancedClosed: advanced ? !advanced.open : null,
        hasCatalogUrl: Object.prototype.hasOwnProperty.call(S.settings || {}, 'catalogUrl'),
        logsPathField: (document.getElementById('set-logs-path') || {}).value || ''
      };
    })()`);
    assert(!settings.hasCatalogInput, 'catalog URL input is still visible');
    assert(!settings.hasDataDirInput, 'data dir input is still visible');
    assert(settings.tabCount === 0, 'settings tabs are still visible');
    assert(settings.hasAdvanced, 'advanced settings section is missing');
    assert(settings.advancedClosed, 'advanced settings must be closed by default');
    assert(!settings.hasCatalogUrl, 'catalogUrl must not be stored in settings');
    assert(settings.logsPathField, 'logs path is empty');
    console.log('PASS: internal settings are hidden');
    await screenshot(cdp, path.join(artifacts, 'settings.png'));

    const logsPath = await evaluate(cdp, `api.getLogsPath()`);
    assert(logsPath && fs.existsSync(logsPath), `log file is missing: ${logsPath}`);
    console.log('PASS: logging is enabled');

    await evaluate(cdp, `closeModals()`);

    const detail = await evaluate(cdp, `(async () => {
      const mode = S.modes[0];
      const installed = isInstalled(mode);
      openDetail(mode.id);
      await new Promise((resolve) => setTimeout(resolve, 500));
      const values = [...document.querySelectorAll('#detail-meta .meta-value')];
      return {
        title: document.getElementById('detail-title').textContent,
        metaCount: document.querySelectorAll('#detail-meta .meta-item').length,
        truncated: values.filter((value) => value.scrollWidth > value.clientWidth + 1).map((value) => value.textContent),
        folderHidden: document.getElementById('side-folder').hidden,
        installed
      };
    })()`);
    assert(detail.title && detail.title.trim(), 'detail title is empty');
    assert(detail.metaCount >= 8, 'detail meta is incomplete');
    assert(detail.truncated.length === 0, `detail meta values are truncated: ${detail.truncated.join(', ')}`);
    assert(detail.folderHidden === !detail.installed, 'open-folder button visibility does not match installed state');
    console.log('PASS: detail page fits and folder button is correct');
    await screenshot(cdp, path.join(artifacts, 'detail.png'));

    await evaluate(cdp, `showStore()`);
    await sleep(300);
    await screenshot(cdp, path.join(artifacts, 'store.png'));
    console.log('PASS: store screenshot captured');

    console.log(`SMOKE OK (artifacts: ${artifacts})`);
  } catch (error) {
    fail(error.message);
    console.error(error.stack || '');
  } finally {
    if (cdp) {
      try { cdp.ws.close(); } catch {}
    }
    if (child && child.pid) killTree(child.pid);
  }

  process.exit(process.exitCode || 0);
})();
