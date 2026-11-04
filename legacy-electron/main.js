'use strict';

const { app, BrowserWindow, ipcMain, shell, dialog } = require('electron');
const path = require('path');
const fs = require('fs');
const crypto = require('crypto');
const Store = require('./store');
const auth = require('./auth');
const launcher = require('./launcher');
const publisher = require('./publisher');

const PREVIEW = process.env.TDEV_PREVIEW === '1';
if (PREVIEW) app.disableHardwareAcceleration();

let win = null;
let store = null;

function send(channel, payload) {
  if (win && !win.isDestroyed()) win.webContents.send(channel, payload);
}

function createWindow() {
  const s = store.settings;
  // размер окна лаунчера — отдельные настройки (width/height — это окно игры),
  // раньше лаунчер открывался с размерами окна игры и «прыгал» до минимальных
  const lw = Math.max(980, Number(s.launcherWidth) || 1200);
  const lh = Math.max(640, Number(s.launcherHeight) || 740);
  win = new BrowserWindow({
    width: lw,
    height: lh,
    minWidth: 980,
    minHeight: 640,
    frame: false,
    backgroundColor: '#0a0a13',
    show: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  });

  win.loadFile(
    path.join(__dirname, 'src', 'index.html'),
    PREVIEW && process.env.TDEV_PREVIEW_PAGE ? { query: { open: process.env.TDEV_PREVIEW_PAGE } } : undefined
  );
  win.once('ready-to-show', () => win.show());

  // запоминаем размер окна лаунчера при закрытии
  win.on('close', () => {
    try {
      if (win && !win.isDestroyed() && !win.isMaximized()) {
        const [w, h] = win.getSize();
        store.saveSettings({ launcherWidth: w, launcherHeight: h });
      }
    } catch {}
  });

  win.on('maximize', () => send('win:maximized', true));
  win.on('unmaximize', () => send('win:maximized', false));

  // Служебный режим: скриншот интерфейса для отладки, затем выход
  if (PREVIEW) {
    win.webContents.once('did-finish-load', async () => {
      await new Promise((r) => setTimeout(r, 2500));
      try {
        const img = await win.webContents.capturePage();
        fs.writeFileSync(path.join(__dirname, 'preview.png'), img.toPNG());
      } catch (err) {
        console.error('preview failed:', err);
      }
      app.exit(0);
    });
  }
}

// --- Список режимов: живёт ТОЛЬКО в GitHub репо (modes.json + arts/ + modes/<id>/) ---

function remoteCachePath() {
  return path.join(app.getPath('userData'), 'remote-modes.json');
}

function readCachedRemoteModes() {
  try {
    return JSON.parse(fs.readFileSync(remoteCachePath(), 'utf8')).modes || [];
  } catch {
    return [];
  }
}

async function fetchRemoteModes(repo) {
  let notFound = false;
  for (const branch of ['main', 'master']) {
    let res;
    try {
      res = await fetch(`https://raw.githubusercontent.com/${repo}/${branch}/modes.json`, {
        signal: AbortSignal.timeout(6000)
      });
    } catch {
      throw new Error('network'); // вызывающий возьмёт оффлайн-кэш
    }
    if (res.status === 404) {
      notFound = true;
      continue;
    }
    if (!res.ok) continue;
    const data = await res.json().catch(() => null);
    if (data && Array.isArray(data.modes)) {
      try {
        fs.writeFileSync(remoteCachePath(), JSON.stringify({ fetchedAt: Date.now(), modes: data.modes }));
      } catch {}
      return data.modes;
    }
  }
  if (notFound) {
    // modes.json нет в репо — список пуст, оффлайн-кэш сбрасываем
    try {
      fs.writeFileSync(remoteCachePath(), JSON.stringify({ fetchedAt: Date.now(), modes: [] }));
    } catch {}
    return [];
  }
  throw new Error('bad response');
}

let modesCache = null;
const sessionPublished = []; // опубликованное в этом сеансе: raw CDN может кэшировать до 5 минут

async function loadModes() {
  if (modesCache) return modesCache;
  const repo = store && store.settings.githubRepo;
  let remote = [];
  if (repo) {
    try {
      remote = await fetchRemoteModes(repo);
    } catch {
      remote = readCachedRemoteModes();
    }
  } else {
    remote = readCachedRemoteModes();
  }
  const map = new Map();
  for (const m of remote) map.set(m.id, m);
  for (const m of sessionPublished) map.set(m.id, { ...(map.get(m.id) || {}), ...m });
  modesCache = [...map.values()];
  return modesCache;
}

/** Запоминаем свежеопубликованный режим в списке + в оффлайн-кэше. */
function rememberPublished(entry) {
  const upsert = (list) => {
    const i = list.findIndex((m) => m.id === entry.id);
    if (i >= 0) list[i] = { ...list[i], ...entry };
    else list.push(entry);
    return list;
  };
  upsert(sessionPublished);
  try {
    fs.writeFileSync(remoteCachePath(), JSON.stringify({ fetchedAt: Date.now(), modes: upsert(readCachedRemoteModes()) }));
  } catch {}
  modesCache = null;
}

app.whenReady().then(() => {
  store = new Store(app.getPath('userData'), path.join(app.getPath('appData'), '.tdevlauncher'));

  // --- Окно ---
  ipcMain.handle('win:minimize', () => win.minimize());
  ipcMain.handle('win:toggleMax', () => (win.isMaximized() ? win.unmaximize() : win.maximize()));
  ipcMain.handle('win:close', () => win.close());

  // --- Данные ---
  ipcMain.handle('modes:get', () => loadModes());
  ipcMain.handle('modes:refresh', () => {
    modesCache = null; // принудительно перечитываем из репо (raw CDN может кэшировать ~5 минут)
    return loadModes();
  });
  ipcMain.handle('state:get', () => store.state);
  ipcMain.handle('settings:get', () => store.settings);
  ipcMain.handle('settings:save', (e, patch) => {
    store.saveSettings(patch);
    return store.settings;
  });
  ipcMain.handle('settings:chooseDir', async () => {
    const r = await dialog.showOpenDialog(win, { properties: ['openDirectory', 'createDirectory'] });
    return r.canceled ? null : r.filePaths[0];
  });
  ipcMain.handle('shell:open', (e, url) => {
    if (typeof url === 'string' && /^https?:\/\//.test(url)) shell.openExternal(url);
  });

  // --- Аккаунты ---
  ipcMain.handle('accounts:get', () => store.accounts);
  ipcMain.handle('accounts:current', () => store.getCurrentAccount());
  ipcMain.handle('accounts:login', async (e, { username, password }) => {
    try {
      if (!username || !password) return { ok: false, error: 'Введите логин и пароль' };
      const clientToken = store.getClientToken();
      const data = await auth.elyLogin(String(username).trim(), String(password), clientToken);
      const profile = data.selectedProfile || (data.availableProfiles && data.availableProfiles[0]) || null;
      const acc = {
        id: crypto.randomUUID(),
        type: 'ely',
        name: profile ? profile.name : String(username).trim(),
        uuid: profile ? profile.id : auth.offlineUuid(String(username).trim()),
        accessToken: data.accessToken,
        clientToken: data.clientToken || clientToken,
        addedAt: Date.now()
      };
      store.upsertAccount(acc);
      store.setCurrentAccount(acc.id);
      return { ok: true, account: acc };
    } catch (err) {
      return { ok: false, error: err.message || 'Ошибка входа' };
    }
  });
  ipcMain.handle('accounts:offline', (e, { name }) => {
    const nick = String(name || '').trim();
    if (!/^[A-Za-z0-9_А-Яа-яЁё-]{3,16}$/.test(nick)) {
      return { ok: false, error: 'Ник: 3–16 символов, без пробелов' };
    }
    const acc = {
      id: crypto.randomUUID(),
      type: 'offline',
      name: nick,
      uuid: auth.offlineUuid(nick),
      accessToken: '0',
      clientToken: store.getClientToken(),
      addedAt: Date.now()
    };
    store.upsertAccount(acc);
    store.setCurrentAccount(acc.id);
    return { ok: true, account: acc };
  });
  ipcMain.handle('accounts:select', (e, id) => {
    store.setCurrentAccount(id);
    return store.getCurrentAccount();
  });
  ipcMain.handle('accounts:remove', (e, id) => {
    store.removeAccount(id);
    return { accounts: store.accounts, current: store.getCurrentAccount() };
  });
  ipcMain.handle('accounts:refresh', async () => {
    const acc = store.getCurrentAccount();
    if (!acc || acc.type !== 'ely') return store.getCurrentAccount();
    try {
      const ok = await auth.elyValidate(acc.accessToken, acc.clientToken);
      if (!ok) {
        const data = await auth.elyRefresh(acc.accessToken, acc.clientToken);
        acc.accessToken = data.accessToken;
        if (data.selectedProfile) acc.name = data.selectedProfile.name;
        delete acc.sessionError;
        store.saveAccounts();
      }
    } catch (err) {
      acc.sessionError = true;
      store.saveAccounts();
    }
    return store.getCurrentAccount();
  });

  // --- Игра ---
  ipcMain.handle('game:install', (e, modeId) => startGame(modeId, 'install'));
  ipcMain.handle('game:play', (e, modeId) => startGame(modeId, 'play'));
  ipcMain.handle('game:cancel', () => launcher.cancel());
  ipcMain.handle('game:openFolder', () => {
    fs.mkdirSync(store.settings.dir, { recursive: true });
    shell.openPath(store.settings.dir);
  });

  // --- Публикация режимов (кнопка «+») ---
  ipcMain.handle('publish:chooseImage', async () => {
    const r = await dialog.showOpenDialog(win, {
      properties: ['openFile'],
      filters: [{ name: 'Изображения', extensions: ['png', 'jpg', 'jpeg', 'svg', 'webp'] }]
    });
    return r.canceled ? null : r.filePaths[0];
  });
  ipcMain.handle('publish:chooseZip', async () => {
    const r = await dialog.showOpenDialog(win, {
      properties: ['openFile'],
      filters: [{ name: 'Архив ZIP', extensions: ['zip'] }]
    });
    return r.canceled ? null : r.filePaths[0];
  });
  ipcMain.handle('publish:readPreview', (e, p) => {
    try {
      if (fs.statSync(p).size > 8 * 1024 * 1024) return null;
      const ext = path.extname(p).slice(1).toLowerCase();
      const mime = ext === 'svg' ? 'image/svg+xml' : ext === 'jpg' || ext === 'jpeg' ? 'image/jpeg' : `image/${ext}`;
      return `data:${mime};base64,${fs.readFileSync(p).toString('base64')}`;
    } catch {
      return null;
    }
  });
  ipcMain.handle('publish:statFile', (e, p) => {
    try {
      const size = fs.statSync(p).size;
      return { size, human: publisher.humanSize(size) };
    } catch {
      return { size: 0, human: '—' };
    }
  });
  ipcMain.handle('publish:mode', async (e, { form, imagePath, zipPath }) => {
    try {
      form.repo = String(form.repo || '').trim();
      form.token = String(form.token || '').trim();
      store.saveSettings({ githubRepo: form.repo, githubToken: form.token });

      const entry = await publisher.publish(
        {
          repo: form.repo,
          token: form.token,
          title: String(form.title || '').trim(),
          version: String(form.version || '').trim(),
          mcVersion: String(form.mcVersion || '').trim(),
          loader: String(form.loader || 'vanilla'),
          loaderVersion: form.loaderVersion || null,
          tags: Array.isArray(form.tags) ? form.tags : [],
          description: form.description || '',
          changelog: form.changelog || '',
          accent: form.accent || '#7c5cff',
          imagePath,
          zipPath
        },
        {
          onEvent: (type, payload) => send('publish:event', { type, payload }),
          tmpDir: path.join(app.getPath('temp'), `tdev-publish-${Date.now()}`)
        }
      );

      // ничего не храним локально: арты/modes.json/сборка живут только в репо
      entry.image = null;
      entry.imageUrl = entry.imageUrl || null;
      entry.build = entry.build || { files: 0, bytes: 0 };
      rememberPublished(entry);

      send('publish:event', { type: 'done', payload: null });
      return { ok: true, mode: entry };
    } catch (err) {
      send('publish:event', { type: 'error', payload: String((err && err.message) || err) });
      return { ok: false, error: (err && err.message) || 'Ошибка публикации' };
    }
  });

  async function startGame(modeId, kind) {
    if (launcher.isActive()) return { ok: false, error: 'Дождитесь завершения текущей операции' };
    const mode = (await loadModes()).find((m) => m.id === modeId);
    if (!mode) return { ok: false, error: 'Режим не найден' };
    const acc = store.getCurrentAccount();
    if (!acc) return { ok: false, error: 'Сначала войдите в аккаунт' };
    const settings = store.settings;
    fs.mkdirSync(settings.dir, { recursive: true });

    launcher.start({
      mode,
      account: acc,
      settings,
      installOnly: kind === 'install',
      onEvent: (type, payload) => {
        if (type === 'installed') store.setModeInstalled(modeId, mode.version);
        send('game:event', { modeId, type, payload });
      }
    });
    return { ok: true };
  }

  createWindow();
});

app.on('window-all-closed', () => app.quit());
