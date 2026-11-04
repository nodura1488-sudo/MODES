'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const extract = require('extract-zip');

const API = 'https://api.github.com';
const RAW = 'https://raw.githubusercontent.com';

function ghHeaders(token, extra = {}) {
  return {
    Authorization: `Bearer ${token}`,
    Accept: 'application/vnd.github+json',
    'User-Agent': 'TDEV-Launcher',
    'X-GitHub-Api-Version': '2022-11-28',
    'Content-Type': 'application/json',
    ...extra
  };
}

async function ghJson(url, token, options = {}) {
  const res = await fetch(url, { ...options, headers: ghHeaders(token, options.headers) });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    const err = new Error(data.message || `GitHub API ${res.status}`);
    err.status = res.status;
    throw err;
  }
  return data;
}

const TRANSLIT = {
  а: 'a', б: 'b', в: 'v', г: 'g', д: 'd', е: 'e', ё: 'e', ж: 'zh', з: 'z', и: 'i',
  й: 'y', к: 'k', л: 'l', м: 'm', н: 'n', о: 'o', п: 'p', р: 'r', с: 's', т: 't',
  у: 'u', ф: 'f', х: 'h', ц: 'c', ч: 'ch', ш: 'sh', щ: 'sch', ъ: '', ы: 'y',
  ь: '', э: 'e', ю: 'yu', я: 'ya'
};

function slugify(title) {
  const s = String(title)
    .toLowerCase()
    .split('')
    .map((ch) => (TRANSLIT[ch] !== undefined ? TRANSLIT[ch] : ch))
    .join('');
  return s.replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'mode';
}

function humanSize(bytes) {
  if (!bytes && bytes !== 0) return '—';
  if (bytes < 1024 * 1024) return Math.max(1, Math.round(bytes / 1024)) + ' КБ';
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(bytes < 10 * 1024 * 1024 ? 1 : 0) + ' МБ';
  return (bytes / 1024 / 1024 / 1024).toFixed(1) + ' ГБ';
}

function upsertMode(list, entry) {
  const idx = list.findIndex((m) => m.id === entry.id);
  if (idx >= 0) {
    list[idx] = { ...list[idx], ...entry };
    return [...list];
  }
  return [...list, entry];
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

/** Git blob sha1 — чтобы не перезаливать файлы, которые не менялись. */
function gitBlobSha(filePath) {
  const size = fs.statSync(filePath).size;
  const hash = crypto.createHash('sha1');
  hash.update(`blob ${size}\0`);
  return new Promise((resolve, reject) => {
    fs.createReadStream(filePath)
      .on('data', (c) => hash.update(c))
      .on('end', () => resolve(hash.digest('hex')))
      .on('error', reject);
  });
}

function encPath(p) {
  return p.split('/').map(encodeURIComponent).join('/');
}

function isJunk(name) {
  return name === '.DS_Store' || name === 'Thumbs.db' || name === '__MACOSX';
}

/** Рекурсивный обход папки: [{ abs, rel }] */
function walk(dir, baseDir, out = []) {
  for (const name of fs.readdirSync(dir)) {
    if (isJunk(name)) continue;
    const abs = path.join(dir, name);
    const st = fs.statSync(abs);
    if (st.isDirectory()) walk(abs, baseDir, out);
    else out.push({ abs, rel: path.relative(baseDir, abs).split(path.sep).join('/') });
  }
  return out;
}

async function putFile(repo, branch, token, repoPath, absPath, existingShas) {
  const gitSha = await gitBlobSha(absPath);
  if (existingShas[repoPath] === gitSha) return 'skip'; // файл не менялся — не льём

  const content = fs.readFileSync(absPath).toString('base64');
  const body = {
    message: `build: ${repoPath}`,
    content,
    branch
  };
  if (existingShas[repoPath]) body.sha = existingShas[repoPath];

  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      await ghJson(`${API}/repos/${repo}/contents/${encPath(repoPath)}`, token, {
        method: 'PUT',
        body: JSON.stringify(body)
      });
      return 'ok';
    } catch (err) {
      if (attempt === 3) throw err;
      await new Promise((r) => setTimeout(r, 800 * attempt));
    }
  }
}

async function deleteFile(repo, branch, token, repoPath, sha) {
  await ghJson(`${API}/repos/${repo}/contents/${encPath(repoPath)}`, token, {
    method: 'DELETE',
    body: JSON.stringify({ message: `build: remove ${repoPath}`, sha, branch })
  }).catch(() => {});
}

/**
 * Публикация режима: zip сборки распаковывается, каждый файл заливается
 * в репозиторий как modes/<id>/<файл> + modes/<id>/manifest.json (sha256).
 * Благодаря манифесту у игроков обновления качаются инкрементально:
 * новый мод скачается, удалённый удалится, конфиг перезапишется.
 */
async function publish(payload, { onEvent, tmpDir }) {
  const {
    repo, token, title, version, mcVersion,
    tags = [], description = '', changelog = '', accent = '#7c5cff',
    loader = 'vanilla', loaderVersion = null,
    imagePath, zipPath
  } = payload;

  onEvent('step', 'Проверяю данные…');

  if (!/^[\w.-]+\/[\w.-]+$/.test(repo)) throw new Error('Репозиторий в формате owner/repo, например nodura1488-sudo/MODES');
  if (!token) throw new Error('Нужен GitHub токен');
  if (!title) throw new Error('Введи название режима');
  if (!/^[\w.-]+$/.test(version)) throw new Error('Версия: только буквы, цифры, точки и дефисы');
  if (!/^[\w.-]+$/.test(mcVersion)) throw new Error('Версия Minecraft указана неверно');
  if (!['vanilla', 'forge', 'fabric'].includes(loader)) throw new Error('Загрузчик: vanilla, forge или fabric');

  const id = slugify(title);
  const buildPrefix = `modes/${id}`;

  // --- распаковка сборки и построение манифеста ---
  let files = {}; // rel path -> { sha256, size }
  let totalBytes = 0;

  if (zipPath) {
    onEvent('step', 'Читаю сборку…');
    fs.mkdirSync(tmpDir, { recursive: true });
    await extract(zipPath, { dir: tmpDir });
    const list = walk(tmpDir, tmpDir);
    onEvent('step', `Считаю хэши (${list.length} файлов)…`);
    for (const f of list) {
      if (f.rel === 'manifest.json') continue; // служебный файл, генерируем сами
      const st = fs.statSync(f.abs);
      if (st.size > 95 * 1024 * 1024) {
        throw new Error(`Файл больше 95 МБ: ${f.rel}. GitHub Contents API не примет — убери или залей вручную`);
      }
      files[f.rel] = { sha256: await sha256File(f.abs), size: st.size };
      totalBytes += st.size;
    }
  }

  const manifest = {
    mode: id,
    version,
    generatedAt: new Date().toISOString(),
    files
  };

  // --- данные репозитория ---
  onEvent('step', 'Читаю репозиторий…');
  const repoInfo = await ghJson(`${API}/repos/${repo}`, token);
  const branch = repoInfo.default_branch || 'main';

  // дерево репо одним запросом: путь -> blob sha
  // 409 = репо пустой (без коммитов) — начинаем с нуля, первый PUT создаст ветку
  let existingShas = {};
  try {
    const tree = await ghJson(`${API}/repos/${repo}/git/trees/${encodeURIComponent(branch)}?recursive=1`, token);
    for (const item of tree.tree || []) {
      if (item.type === 'blob') existingShas[item.path] = item.sha;
    }
  } catch (err) {
    if (err.status !== 409 && err.status !== 404) throw err;
  }

  // файлы, которые были в репо, но исчезли из новой сборки — удалить
  const oldBuildPaths = Object.keys(existingShas).filter((p) => p.startsWith(`${buildPrefix}/`));
  const newPaths = new Set([...Object.keys(files), 'manifest.json']);
  const toDelete = oldBuildPaths.filter((p) => !newPaths.has(p.slice(buildPrefix.length + 1)));

  // --- заливка файлов ---
  const fileEntries = Object.entries(files);
  let uploaded = 0;
  for (const [rel, meta] of fileEntries) {
    uploaded++;
    onEvent('step', `Загрузка ${uploaded}/${fileEntries.length}: ${rel}`);
    onEvent('progress', { task: uploaded - 1, total: fileEntries.length });
    await putFile(repo, branch, token, `${buildPrefix}/${rel}`, path.join(tmpDir, rel), existingShas);
  }
  onEvent('progress', { task: fileEntries.length, total: fileEntries.length });

  // --- манифест ---
  onEvent('step', 'Загружаю манифест…');
  const manifestTmp = path.join(tmpDir, 'manifest.json');
  fs.mkdirSync(path.dirname(manifestTmp), { recursive: true });
  fs.writeFileSync(manifestTmp, JSON.stringify(manifest, null, 2));
  await putFile(repo, branch, token, `${buildPrefix}/manifest.json`, manifestTmp, existingShas);

  // --- удаление стертых файлов ---
  if (toDelete.length) {
    onEvent('step', `Удаляю устаревшие файлы (${toDelete.length})…`);
    for (const repoPath of toDelete) {
      await deleteFile(repo, branch, token, repoPath, existingShas[repoPath]);
    }
  }

  // --- арт ---
  let imageUrl = null;
  if (imagePath) {
    onEvent('step', 'Загружаю арт…');
    const ext = path.extname(imagePath).toLowerCase().slice(1) || 'png';
    const artPath = `arts/${id}.${ext}`;
    await putFile(repo, branch, token, artPath, imagePath, existingShas);
    imageUrl = `${RAW}/${repo}/${branch}/${encPath(artPath)}`;
  }

  // --- modes.json ---
  onEvent('step', 'Обновляю modes.json…');
  let modesSha = null;
  let remoteModes = [];
  try {
    const contents = await ghJson(`${API}/repos/${repo}/contents/modes.json?ref=${encodeURIComponent(branch)}`, token);
    modesSha = contents.sha;
    remoteModes = JSON.parse(Buffer.from(contents.content, 'base64').toString('utf8')).modes || [];
  } catch (err) {
    if (err.status !== 404) throw err;
  }

  const entry = {
    id,
    title,
    mcVersion,
    version,
    loader,
    loaderVersion: loader !== 'vanilla' ? (loaderVersion || null) : null,
    size: zipPath ? humanSize(totalBytes) : '—',
    players: '—',
    accent,
    tags: tags.length ? tags : ['Режим'],
    image: '',
    imageUrl,
    build: { files: Object.keys(files).length, bytes: totalBytes, version },
    description,
    changelog
  };
  const merged = upsertMode(remoteModes, entry);

  const modesTmp = path.join(tmpDir, 'modes.json');
  fs.writeFileSync(modesTmp, JSON.stringify({ modes: merged }, null, 2));
  await putFile(repo, branch, token, 'modes.json', modesTmp, existingShas);

  // --- убрать tmp ---
  try {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  } catch {}

  return entry;
}

module.exports = { publish, slugify, humanSize, upsertMode };
