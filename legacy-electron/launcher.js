'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { pipeline } = require('stream/promises');
const { Readable } = require('stream');
const { Client } = require('minecraft-launcher-core');

const API = 'https://api.github.com';
const RAW = 'https://raw.githubusercontent.com';

let active = null; // { modeId, installOnly, child, onEvent, finished, spawned, abort }
let branchCache = {}; // repo -> branch

function isActive() {
  return !!active;
}

function cancel() {
  if (!active) return;
  const a = active;
  active = null;
  a.finished = true;
  try {
    if (a.child && a.child.pid && !a.child.killed) a.child.kill('SIGKILL');
  } catch {}
  try {
    if (a.abort) a.abort.abort();
  } catch {}
  a.onEvent('error', 'Операция отменена');
}

function emit(type, payload) {
  if (active && !active.finished) active.onEvent(type, payload);
}

function finishInstall() {
  if (!active || active.finished) return;
  const a = active;
  active = null;
  a.finished = true;
  a.onEvent('installed', null);
}

function abortIfCancelled() {
  return !active;
}

/**
 * Точка входа. Пайплайн:
 *   сборка (sync по манифесту) → загрузчик (Forge/Fabric) → запуск (mlc)
 * installOnly для сборочных режимов — без mlc (java не нужна),
 * для чисто ванильных — mlc скачивает версию и глушится на старте java.
 */
function start({ mode, account, settings, installOnly, onEvent }) {
  if (active) return;
  active = {
    modeId: mode.id, installOnly, child: null, onEvent,
    finished: false, spawned: false, abort: new AbortController()
  };
  runFlow({ mode, account, settings, installOnly }).catch((err) => {
    if (!active) return;
    const str = String((err && err.message) || err);
    const a = active;
    active = null;
    a.finished = true;
    a.onEvent('error', /abort/i.test(str) ? 'Операция отменена' : str);
  });
}

/** Папка режима: <игра>/games/<id>/. Старые установки (<игра>/<id>) переезжают сами. */
function resolveGameDir(mode, settings) {
  const newDir = path.join(settings.dir, 'games', mode.id);
  const oldDir = path.join(settings.dir, mode.id);
  if (fs.existsSync(oldDir) && !fs.existsSync(newDir)) {
    fs.mkdirSync(path.dirname(newDir), { recursive: true });
    try {
      fs.renameSync(oldDir, newDir);
    } catch {
      return oldDir; // занят запущенной игрой — работаем по-старому
    }
  }
  return newDir;
}

async function runFlow({ mode, account, settings, installOnly }) {
  const gameDir = resolveGameDir(mode, settings);
  fs.mkdirSync(gameDir, { recursive: true });

  const hasBuild = mode.build && mode.build.files > 0;
  const manifestPath = path.join(gameDir, '.manifest.json');
  const loader = mode.loader && mode.loader !== 'vanilla' ? mode.loader : null;

  // 1) Сборка
  if (hasBuild) {
    if (!fs.existsSync(manifestPath) || installOnly) {
      await syncBuild(mode, settings, gameDir);
      if (abortIfCancelled()) return;
    }
    if (installOnly) {
      // файлы сборки на месте; файлы Minecraft докачаются при первом запуске
      if (loader) {
        await ensureLoader(mode, settings);
        if (abortIfCancelled()) return;
      }
      finishInstall();
      return;
    }
  }

  // 2) Загрузчик
  let loaderInfo = null;
  if (loader) {
    loaderInfo = await ensureLoader(mode, settings);
    if (abortIfCancelled()) return;
  }

  // 3) Файлы Minecraft + игра
  if (installOnly && !hasBuild) {
    await verifyJava(settings);
    if (abortIfCancelled()) return;
    startVanilla({ mode, account, settings, installOnly: true, loaderInfo });
  } else {
    await verifyJava(settings);
    if (abortIfCancelled()) return;
    startVanilla({ mode, account, settings, installOnly: false, loaderInfo, gameDir });
  }
}

/** Проверка Java с таймаутом — mlc делает это без таймаута и может висеть вечно. */
function verifyJava(settings) {
  return new Promise((resolve) => {
    emit('status', 'Проверяю Java…');
    const java = settings.javaPath || 'java';
    require('child_process').exec(`"${java}" -version`, { timeout: 15000 }, (error, _stdout, stderr) => {
      if (error) {
        const why = error.killed ? 'не отвечает (таймаут)' : String(error.message || error).split('\n')[0];
        failFlow(`Java не найдена: ${why}. Установи Java 17+ (или укажи путь в настройках)`);
        resolve(false);
        return;
      }
      resolve(true);
    });
  });
}

function failFlow(msg) {
  if (!active) return;
  const a = active;
  active = null;
  a.finished = true;
  a.onEvent('error', msg);
}

// ---------------------------------------------------------------- сборка (modes/<id>/)

/** Инкрементальное обновление: качаем только изменённые файлы, лишние удаляем. */
async function syncBuild(mode, settings, gameDir) {
  emit('status', 'Проверяю обновления…');

  const remote = await fetchManifest(mode, settings);
  const remoteFiles = remote.files || {};

  const manifestPath = path.join(gameDir, '.manifest.json');
  const local = readJson(manifestPath) || { files: {} };
  const localFiles = local.files || {};

  const toDownload = [];
  for (const [rel, meta] of Object.entries(remoteFiles)) {
    const lf = localFiles[rel];
    // без sha256 (режим залит руками) сравниваем только по составу файлов
    const same = lf && (!meta.sha256 || lf.sha256 === meta.sha256);
    if (!same) toDownload.push(rel);
  }
  const toDelete = Object.keys(localFiles).filter((rel) => !remoteFiles[rel]);

  if (!toDownload.length && !toDelete.length) {
    emit('status', 'Сборка актуальна');
    writeJson(manifestPath, { mode: mode.id, version: mode.version, files: remoteFiles });
    return;
  }

  const totalBytes = toDownload.reduce((s, rel) => s + (remoteFiles[rel].size || 0), 0);
  let doneBytes = 0;

  for (const rel of toDownload) {
    emit('status', `Скачивание: ${rel}`);
    const dest = safeJoin(gameDir, rel);
    if (!dest) throw new Error(`Опасный путь в манифесте: ${rel}`);
    const url = `${RAW}/${settings.githubRepo}/${branch(settings.githubRepo)}/${encPath(`modes/${mode.id}/${rel}`)}`;
    await downloadFile(url, dest, settings.githubToken, active.abort.signal, (done, total) => {
      emit('progress', { task: doneBytes + done, total: totalBytes || doneBytes + done });
    });
    // проверка хэша — только если манифест настоящий (залит через лаунчер)
    if (remoteFiles[rel].sha256) {
      const hash = await sha256File(dest);
      if (hash !== remoteFiles[rel].sha256) {
        throw new Error(`Файл повреждён при скачивании: ${rel}`);
      }
    }
    doneBytes += remoteFiles[rel].size || 0;
  }

  if (toDelete.length) {
    emit('status', `Удаляю устаревшие файлы (${toDelete.length})…`);
    for (const rel of toDelete) {
      const p = safeJoin(gameDir, rel);
      if (p) {
        try { fs.unlinkSync(p); } catch {}
      }
    }
    pruneEmptyDirs(gameDir);
  }

  writeJson(manifestPath, { mode: mode.id, version: mode.version, files: remoteFiles });
}

// ---------------------------------------------------------------- загрузчики (Forge / Fabric)

function parseMcVersion(mc) {
  const parts = String(mc).split('.').map((n) => parseInt(n, 10) || 0);
  return { major: parts[0] || 0, minor: parts[1] || 0, patch: parts[2] || 0 };
}

async function fetchJson(url, signal) {
  const res = await fetch(url, { signal: signal || AbortSignal.timeout(20000) });
  if (!res.ok) throw new Error(`Ошибка запроса ${new URL(url).host} (${res.status})`);
  return res.json();
}

async function forgePromoVersion(mc) {
  const data = await fetchJson('https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json');
  const promos = data.promos || {};
  const v = promos[`${mc}-recommended`] || promos[`${mc}-latest`];
  if (!v) throw new Error(`Forge не поддерживает Minecraft ${mc}`);
  return v;
}

async function fabricLatestVersion(mc) {
  const list = await fetchJson(`https://meta.fabricmc.net/v2/versions/loader/${mc}`);
  const stable = list.find((x) => x.loader && x.loader.stable) || list[0];
  if (!stable || !stable.loader) throw new Error(`Fabric не поддерживает Minecraft ${mc}`);
  return stable.loader.version;
}

/** Скачивает/подготавливает файлы загрузчика. Возвращает инфо для mlc. */
async function ensureLoader(mode, settings) {
  const mc = mode.mcVersion;

  if (mode.loader === 'forge') {
    const fv = mode.loaderVersion || await forgePromoVersion(mc);
    const forgeId = `${mc}-${fv}`;
    const old = parseMcVersion(mc).minor <= 12; // ≤1.12.x — universal jar
    const fileName = `forge-${forgeId}-${old ? 'universal' : 'installer'}.jar`;
    const destDir = path.join(settings.dir, 'forge');
    fs.mkdirSync(destDir, { recursive: true });
    const dest = path.join(destDir, fileName);

    if (!fs.existsSync(dest)) {
      emit('status', `Скачивание Forge ${fv}…`);
      const url = `https://maven.minecraftforge.net/net/minecraftforge/forge/${forgeId}/${fileName}`;
      await downloadFile(url, dest, null, active.abort.signal, (done, total) => {
        emit('progress', { task: done, total: total || done });
      });
    }
    emit('status', `Установка Forge ${fv}…`);
    return { type: 'forge', installerPath: dest };
  }

  if (mode.loader === 'fabric') {
    const lv = mode.loaderVersion || await fabricLatestVersion(mc);
    const fabricName = `fabric-loader-${lv}-${mc}`;
    const dir = path.join(settings.dir, 'versions', fabricName);
    fs.mkdirSync(dir, { recursive: true });

    const profilePath = path.join(dir, `${fabricName}.json`);
    if (!fs.existsSync(profilePath)) {
      emit('status', `Настройка Fabric ${lv}…`);
      const profile = await fetchJson(
        `https://meta.fabricmc.net/v2/versions/loader/${mc}/${lv}/profile/json`,
        active.abort.signal
      );
      fs.writeFileSync(profilePath, JSON.stringify(profile, null, 2));
    }

    const vanillaJsonPath = path.join(dir, `${mc}.json`);
    if (!fs.existsSync(vanillaJsonPath)) {
      emit('status', 'Получаю данные Minecraft…');
      const manifest = await fetchJson('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json', active.abort.signal);
      const entry = (manifest.versions || []).find((v) => v.id === mc);
      if (!entry) throw new Error(`Версия Minecraft ${mc} не найдена`);
      const vanillaJson = await fetchJson(entry.url, active.abort.signal);
      fs.writeFileSync(vanillaJsonPath, JSON.stringify(vanillaJson, null, 2));
    }

    return { type: 'fabric', customVersion: fabricName };
  }

  return null;
}

// ---------------------------------------------------------------- манифест (GitHub)

/**
 * Манифест сборки.
 * 1) modes/<id>/manifest.json из репо (заливается лаунчером при публикации)
 * 2) fallback: git trees API — режим можно закинуть руками прямо на GitHub,
 *    файлы соберутся по списку, но без хэшей (обновление по составу файлов).
 */
async function fetchManifest(mode, settings) {
  const repo = settings.githubRepo;
  const prefixes = [...new Set([branchCache[repo] || 'main', 'main', 'master'])];
  let lastErr = null;

  for (const br of prefixes) {
    try {
      const url = `${RAW}/${repo}/${br}/${encPath(`modes/${mode.id}/manifest.json`)}`;
      const headers = {};
      if (settings.githubToken) headers.Authorization = `Bearer ${settings.githubToken}`;
      const res = await fetch(url, { headers, signal: AbortSignal.timeout(15000) });
      if (!res.ok) {
        lastErr = new Error(`manifest.json (${res.status})`);
        continue;
      }
      branchCache[repo] = br;
      return await res.json();
    } catch (err) {
      lastErr = err;
    }
  }

  // fallback: собираем список файлов из дерева репозитория
  for (const br of prefixes) {
    try {
      const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'TDEV-Launcher' };
      if (settings.githubToken) headers.Authorization = `Bearer ${settings.githubToken}`;
      const res = await fetch(`${API}/repos/${repo}/git/trees/${encodeURIComponent(br)}?recursive=1`, {
        headers,
        signal: AbortSignal.timeout(15000)
      });
      if (!res.ok) {
        lastErr = new Error(`git trees (${res.status})`);
        continue;
      }
      branchCache[repo] = br;
      const data = await res.json();
      const prefix = `modes/${mode.id}/`;
      const files = {};
      for (const item of data.tree || []) {
        if (item.type !== 'blob' || !item.path.startsWith(prefix)) continue;
        const rel = item.path.slice(prefix.length);
        if (!rel || rel === 'manifest.json') continue;
        files[rel] = { size: item.size || 0 };
      }
      if (Object.keys(files).length) {
        return { mode: mode.id, version: mode.version, files, inferred: true };
      }
      lastErr = new Error(`в репо нет папки modes/${mode.id}/`);
    } catch (err) {
      lastErr = err;
    }
  }

  throw new Error(
    `Сборка «${mode.title}» не найдена в репозитории. ` +
    `Залей файлы в modes/${mode.id}/ или публикуй кнопкой «+». ` +
    (lastErr ? `(${lastErr.message})` : '')
  );
}

function branch(repo) {
  if (branchCache[repo]) return branchCache[repo];
  return 'main';
}

function encPath(p) {
  return p.split('/').map(encodeURIComponent).join('/');
}

function readJson(p) {
  try {
    return JSON.parse(fs.readFileSync(p, 'utf8'));
  } catch {
    return null;
  }
}

function writeJson(p, data) {
  fs.writeFileSync(p, JSON.stringify(data, null, 2), 'utf8');
}

/** Защита от path traversal: путь должен остаться внутри baseDir. */
function safeJoin(baseDir, rel) {
  if (!rel || path.isAbsolute(rel) || rel.includes('..')) return null;
  const dest = path.resolve(baseDir, rel);
  if (!dest.startsWith(path.resolve(baseDir) + path.sep)) return null;
  fs.mkdirSync(path.dirname(dest), { recursive: true });
  return dest;
}

async function downloadFile(url, dest, token, signal, onProgress) {
  const headers = {};
  if (token) headers.Authorization = `Bearer ${token}`;
  const res = await fetch(url, { headers, signal, redirect: 'follow' });
  if (!res.ok) throw new Error(`Ошибка скачивания ${path.basename(dest)} (${res.status})`);

  const total = Number(res.headers.get('content-length')) || 0;
  const part = dest + '.tdev-part';
  const out = fs.createWriteStream(part);
  try {
    await pipeline(
      Readable.fromWeb(res.body),
      async function* (source) {
        let done = 0;
        for await (const chunk of source) {
          done += chunk.length;
          if (onProgress) onProgress(done, total);
          yield chunk;
        }
      },
      out
    );
  } catch (err) {
    try { fs.unlinkSync(part); } catch {}
    throw err;
  }
  fs.renameSync(part, dest);
}

function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    fs.createReadStream(filePath)
      .on('data', (c) => hash.update(c))
      .on('end', () => resolve(hash.digest('hex')))
      .on('error', reject);
  });
}

function pruneEmptyDirs(dir) {
  if (!fs.existsSync(dir)) return;
  for (const name of fs.readdirSync(dir)) {
    if (name === '.manifest.json') continue;
    const p = path.join(dir, name);
    if (fs.statSync(p).isDirectory()) {
      pruneEmptyDirs(p);
      try {
        if (!fs.readdirSync(p).length) fs.rmdirSync(p);
      } catch {}
    }
  }
}

// ---------------------------------------------------------------- ваниль (mlc)

function startVanilla({ mode, account, settings, installOnly, loaderInfo, gameDir }) {
  const versionNumber = mode.versionId || mode.mcVersion;

  const client = new Client();
  active.client = client;
  active.installOnly = installOnly;

  client.on('debug', (msg) => emit('log', String(msg)));
  client.on('data', (msg) => emit('log', String(msg).trim()));
  client.on('progress', (p) => emit('progress', p));
  client.on('download-status', (d) => emit('download-status', d));
  client.on('close', (code) => {
    if (!active) return;
    const a = active;
    active = null;
    a.finished = true;
    if (a.installOnly) {
      const custom = loaderInfo && loaderInfo.customVersion;
      const jar = custom
        ? path.join(settings.dir, 'versions', custom, `${custom}.jar`)
        : path.join(settings.dir, 'versions', versionNumber, `${versionNumber}.jar`);
      if (a.spawned || fs.existsSync(jar)) {
        a.onEvent('installed', null);
      } else {
        a.onEvent('error', 'Java не найдена. Укажи путь к Java в настройках (или установи Java 17+).');
      }
    } else {
      a.onEvent('exit', code);
    }
  });

  const authorization = {
    access_token: account.accessToken || '0',
    client_token: account.clientToken || 'tdev-launcher',
    uuid: account.uuid,
    name: account.name,
    user_properties: '{}'
  };

  const overrides = {};
  if (gameDir) {
    overrides.gameDirectory = gameDir;
    overrides.cwd = gameDir;
  }

  const opts = {
    authorization,
    root: settings.dir,
    version: { number: versionNumber, type: 'release' },
    memory: { max: `${settings.ramGb || 4}G`, min: '512M' },
    window: {
      width: settings.width || 925,
      height: settings.height || 530,
      fullscreen: !!settings.fullscreen
    },
    timeout: 30000,
    overrides
  };

  if (loaderInfo && loaderInfo.type === 'forge') opts.forge = loaderInfo.installerPath;
  if (loaderInfo && loaderInfo.type === 'fabric') opts.version.custom = loaderInfo.customVersion;

  if (settings.javaPath) opts.javaPath = settings.javaPath;

  // client.launch — ASYNC: возвращает Promise<child>, а не сам процесс
  const self = active;
  client
    .launch(opts)
    .then((child) => {
      // за это время операцию могли отменить/заменить — глушим лишний java
      if (active !== self || self.finished) {
        try { if (child && child.pid) child.kill('SIGKILL'); } catch {}
        return;
      }
      self.child = child;
      self.spawned = true;
      if (child) {
        child.on('error', (err) => {
          emit('error', String((err && err.message) || err));
        });
      }
      if (self.installOnly) {
        try { if (child && child.pid) child.kill('SIGKILL'); } catch {}
        finishInstall();
      } else {
        emit('launched', null);
      }
    })
    .catch((err) => {
      if (active !== self || self.finished) return;
      emit('error', String((err && err.message) || err));
    });
}

module.exports = { start, isActive, cancel };
