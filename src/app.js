'use strict';

const api = window.tm;
const $ = (id) => document.getElementById(id);

// WebView2 показывает своё меню (назад, сохранить как, печать) — в лаунчере оно не нужно.
document.addEventListener('contextmenu', (event) => event.preventDefault());

const LOADER_LABELS = {
  vanilla: 'Vanilla',
  forge: 'Forge',
  fabric: 'Fabric',
  quilt: 'Quilt',
  neoforge: 'NeoForge',
  paper: 'Paper',
  purpur: 'Purpur',
  spigot: 'Spigot',
  bukkit: 'Bukkit',
  folia: 'Folia',
  velocity: 'Velocity',
  bungeecord: 'BungeeCord',
  waterfall: 'Waterfall'
};

const UPDATE_TAGS = {
  release: { label: 'Релиз', cls: 'release' },
  hotfix: { label: 'Hot-fix', cls: 'hotfix' },
  content: { label: 'Контент', cls: 'content' },
  balance: { label: 'Баланс', cls: 'balance' },
  fix: { label: 'Исправление', cls: 'fix' },
  tech: { label: 'Техническое', cls: 'tech' }
};

const ART_CACHE = new Map();
const DEFAULT_ACCENT = '#0d8cff';
const SPLASH_START = performance.now();

const S = {
  modes: [],
  state: {},
  settings: null,
  accounts: [],
  current: null,
  selectedId: null,
  featuredId: null,
  view: 'store',
  busy: null,
  running: null,
  progress: 0,
  status: '',
  search: '',
  loading: true,
  refreshing: false,
  elyLogin: null,
  section: 'store',
  detailFrom: null,
  dataDir: '',
  updateInfo: null,
  autoUpdating: false
};

const els = {
  splash: $('splash'),
  splashSub: $('splash-sub'),
  splashBar: $('splash-progress-bar'),
  splashNormal: $('splash-normal'),
  splashUpdate: $('splash-update'),
  splashUpdateVersion: $('splash-update-version'),
  splashUpdateStatus: $('splash-update-status'),
  splashUpdateBar: $('splash-update-bar'),
  splashUpdatePercent: $('splash-update-percent'),
  splashUpdateRing: $('splash-update-ring'),
  sideLibCount: $('side-lib-count'),
  sideInstalledCount: $('side-installed-count'),
  storeView: $('store-view'),
  libraryView: $('library-view'),
  libraryGrid: $('library-grid'),
  libraryCount: $('library-count'),
  libraryEmpty: $('library-empty'),
  libraryEmptyTitle: $('library-empty-title'),
  libraryEmptySub: $('library-empty-sub'),
  detailView: $('detail-view'),
  detailBackLabel: $('detail-back-label'),
  featured: $('featured'),
  featuredImg: $('featured-img'),
  featuredTitle: $('featured-title'),
  featuredTags: $('featured-tags'),
  featuredDesc: $('featured-desc'),
  featuredStatus: $('featured-status'),
  featuredOpen: $('featured-open'),
  modesGrid: $('modes-grid'),
  modesCount: $('modes-count'),
  catalogTitle: $('catalog-title'),
  emptyState: $('empty-state'),
  emptyTitle: $('empty-title'),
  emptySub: $('empty-sub'),
  loadingState: $('loading-state'),
  search: $('search-input'),
  refresh: $('btn-refresh'),
  accChip: $('account-chip'),
  accAvatar: $('account-avatar'),
  accName: $('account-name'),
  updateChip: $('update-chip'),
  updateChipText: $('update-chip-text'),
  updateCurrent: $('update-current'),
  updateLatest: $('update-latest'),
  updateNotes: $('update-notes'),
  btnUpdateDownload: $('btn-update-download'),
  toast: $('toast'),
  detailBack: $('detail-back'),
  detailImg: $('detail-img'),
  detailTitle: $('detail-title'),
  detailTags: $('detail-tags'),
  detailSub: $('detail-sub'),
  detailAction: $('detail-action'),
  detailStatus: $('detail-status'),
  detailProgress: $('detail-progress'),
  detailProgressBar: $('detail-progress-bar'),
  detailMeta: $('detail-meta'),
  detailDesc: $('detail-desc'),
  detailChangelogSection: $('detail-changelog-section'),
  detailChangelog: $('detail-changelog'),
  changelogToggle: $('changelog-toggle'),
  reqSize: $('req-size'),
  sideFolder: $('side-folder'),
  detailSpotlight: $('detail-spotlight'),
  spotlightTitle: $('spotlight-title'),
  spotlightTag: $('spotlight-tag'),
  spotlightDate: $('spotlight-date'),
  spotlightLink: $('spotlight-link'),
  moreSection: $('more-section'),
  moreGrid: $('more-grid'),
  ram: $('set-ram'),
  ramOut: $('ram-out'),
  setDir: $('set-dir'),
  setJava: $('set-java'),
  setWidth: $('set-width'),
  setHeight: $('set-height'),
  setFullscreen: $('set-fullscreen'),
  setJvmFlags: $('set-jvm-flags'),
  setMinimizeOnLaunch: $('set-minimize-on-launch'),
  setDataDir: $('set-data-dir')
};

// ---------------------------------------------------------------- init

init();

async function init() {
  if (api && api.windowSplash) api.windowSplash().catch(() => {});
  bindWindowButtons();
  bindModals();
  bindSidebar();
  bindStore();
  bindDetail();
  bindSettings();
  bindAccountModal();
  bindUpdateModal();

  if (api && api.onGameEvent) api.onGameEvent(onGameEvent);
  if (api && api.getAppDataPath) {
    api.getAppDataPath().then((dir) => {
      S.dataDir = dir || '';
    }).catch(() => {});
  }

  renderStore();
  renderLibrary();
  updateSidebar();
  if (els.splashSub) els.splashSub.textContent = 'Загружаем режимы и аккаунт…';

  try {
    const [modes, state, settings, accounts, current] = await Promise.all([
      api.getModes(),
      api.getState(),
      api.getSettings(),
      api.getAccounts(),
      api.getCurrentAccount()
    ]);
    S.modes = Array.isArray(modes) ? modes : [];
    S.state = state || {};
    S.settings = settings || null;
    S.accounts = Array.isArray(accounts) ? accounts : [];
    S.current = current || null;
    S.loading = false;
    renderAccountChip();
    renderStore();
    renderLibrary();
    updateSidebar();
    requestAnimationFrame(() => { $('app').scrollTop = 0; });
    handleStartupUpdate();

    if (api.refreshSession) {
      api.refreshSession().then((acc) => {
        if (acc) {
          S.current = acc;
          renderAccountChip();
        }
      }).catch(() => {});
    }
  } catch (err) {
    S.loading = false;
    renderStore();
    renderLibrary();
    await finishStartup();
    toast('Не удалось загрузить данные: ' + err, 'error');
  }

  const openParam = new URLSearchParams(location.search).get('open');
  if (openParam && $('modal-' + openParam)) openModal('modal-' + openParam);
}

let splashFinished = false;
function finishSplash() {
  if (splashFinished || S.autoUpdating) return;
  splashFinished = true;
  const elapsed = performance.now() - SPLASH_START;
  const wait = Math.max(0, 680 - elapsed);
  setTimeout(() => {
    if (!els.splash) return;
    if (els.splashBar) {
      els.splashBar.style.animation = 'none';
      els.splashBar.style.width = '100%';
    }
    setTimeout(() => {
      if (!els.splash) return;
      els.splash.classList.add('done');
      setTimeout(() => {
        if (els.splash) els.splash.remove();
      }, 420);
    }, 180);
  }, wait);
}

setTimeout(() => {
  if (!S.autoUpdating) finishStartup();
}, 12000);

// ---------------------------------------------------------------- окно

function bindWindowButtons() {
  $('btn-min').onclick = () => api.minimize();
  $('btn-settings').onclick = () => openModal('modal-settings');
  $('btn-close').onclick = () => api.close();
}

// ---------------------------------------------------------------- store

function bindStore() {
  els.featuredOpen.onclick = () => {
    if (S.featuredId) openDetail(S.featuredId);
  };

  els.search.addEventListener('input', () => {
    S.search = els.search.value.trim().toLowerCase();
    if (S.view === 'detail') {
      S.detailFrom = null;
      showSection(S.section);
    } else {
      renderCurrentSection();
    }
  });

  els.refresh.onclick = async () => {
    if (S.refreshing) return;
    S.refreshing = true;
    els.refresh.classList.add('spinning');
    try {
      const modes = await api.refreshModes();
      S.modes = Array.isArray(modes) ? modes : [];
      S.state = (await api.getState()) || {};
      renderStore();
      renderLibrary();
      if (S.view === 'detail' && S.selectedId) renderDetail();
      toast('Список режимов обновлён', 'ok');
    } catch (err) {
      toast('Не удалось обновить список: ' + err, 'error');
    } finally {
      S.refreshing = false;
      els.refresh.classList.remove('spinning');
    }
  };

  document.addEventListener('click', (e) => {
    if (!e.target.closest('.card-menu') && !e.target.closest('.card-menu-btn')) closeCardMenus();
  });

  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      els.search.focus();
      return;
    }
    if (e.key === 'Escape') {
      if (!document.querySelector('.modal-overlay:not([hidden])')) {
        if (S.view === 'detail') {
          showStore();
        } else if (S.search) {
          els.search.value = '';
          S.search = '';
          renderCurrentSection();
        }
      }
    }
  });
}

function bindSidebar() {
  document.querySelectorAll('.side-link').forEach((btn) => {
    btn.onclick = () => {
      S.detailFrom = null;
      showSection(btn.dataset.section);
    };
  });
}

function showSection(section) {
  S.section = section === 'library' ? 'library' : 'store';
  S.view = S.section;
  els.storeView.hidden = S.section !== 'store';
  els.libraryView.hidden = S.section !== 'library';
  els.detailView.hidden = true;
  updateSidebar();
  if (S.section === 'library') renderLibrary(); else renderStore();
  $('app').scrollTop = 0;
}

function updateSidebar() {
  document.querySelectorAll('.side-link').forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.section === S.section);
  });
}

function renderCurrentSection() {
  if (S.section === 'library') renderLibrary(); else renderStore();
}

function isInstalled(mode) {
  const st = S.state[mode.id];
  return !!(st && st.installed);
}

function inLibrary(mode) {
  const st = S.state[mode.id];
  if (!st) return false;
  if (typeof st.library === 'boolean') return st.library;
  return !!(st.installed && !st.hidden);
}

function isUpdate(mode) {
  const st = S.state[mode.id];
  return !!(st && st.installed && st.version !== mode.version);
}

function cardStatus(mode) {
  if (S.running === mode.id) return { cls: 'playing', text: 'В игре' };
  if (isUpdate(mode)) return { cls: 'update', text: 'Обновление' };
  if (isInstalled(mode)) return { cls: 'installed', text: 'Установлено' };
  if (inLibrary(mode)) return { cls: 'library', text: 'Не установлено' };
  return { cls: 'new', text: 'Доступно' };
}

function loaderLabel(mode) {
  return LOADER_LABELS[mode.loader] || mode.loader || 'Vanilla';
}

function modeSub(mode) {
  return `MC ${mode.mcVersion || '—'} · ${loaderLabel(mode)}`;
}

function cardSub(mode) {
  return `${modeSub(mode)}${mode.version ? ` · v${mode.version}` : ''}`;
}

function modeDescription(mode) {
  return (mode.description || '').trim() || 'Описание пока не заполнено.';
}

function modeTags(mode, limit = 3) {
  const tags = Array.isArray(mode.tags) ? mode.tags.filter(Boolean) : [];
  return tags.slice(0, limit);
}

function sortByTitle(list) {
  return [...list].sort((a, b) => String(a.title || '').localeCompare(String(b.title || ''), 'ru'));
}

function sortModes(list) {
  return [...list].sort((a, b) => {
    const ar = S.running === a.id ? 0 : 1;
    const br = S.running === b.id ? 0 : 1;
    if (ar !== br) return ar - br;
    const ai = isInstalled(a) ? 0 : 1;
    const bi = isInstalled(b) ? 0 : 1;
    if (ai !== bi) return ai - bi;
    return String(a.title || '').localeCompare(String(b.title || ''), 'ru');
  });
}

function matchesSearch(mode) {
  if (!S.search) return true;
  const haystack = [
    mode.title,
    mode.description,
    mode.mcVersion,
    mode.loader,
    ...(Array.isArray(mode.tags) ? mode.tags : [])
  ].join(' ').toLowerCase();
  return haystack.includes(S.search);
}

function filterModes() {
  return S.modes.filter(matchesSearch);
}

function updateSidebarCounts() {
  if (!els.sideLibCount) return;
  els.sideLibCount.textContent = String(S.modes.filter((mode) => inLibrary(mode)).length);
  els.sideInstalledCount.textContent = String(S.modes.filter((mode) => isInstalled(mode)).length);
}

function renderStore() {
  if (!els.storeView) return;

  const searchActive = !!S.search;

  if (S.loading) {
    els.featured.hidden = true;
    els.catalogTitle.textContent = 'Загрузка';
    els.modesCount.textContent = '';
    els.modesGrid.innerHTML = '';
    els.emptyState.hidden = true;
    els.loadingState.hidden = false;
    updateSidebarCounts();
    return;
  }
  els.loadingState.hidden = true;

  if (searchActive) {
    els.featured.hidden = true;
    const result = sortModes(filterModes());
    els.catalogTitle.textContent = 'Результаты поиска';
    els.modesCount.textContent = String(result.length);
    renderModeGrid(els.modesGrid, result);
    setEmpty(
      result.length === 0,
      'Ничего не найдено',
      'Попробуй изменить запрос или обновить список режимов'
    );
    return;
  }

  const featured = S.modes.find((m) => m.imageUrl || m.image) || S.modes[0] || null;
  S.featuredId = featured ? featured.id : null;
  renderFeatured(featured);

  const list = sortModes(S.modes);
  els.catalogTitle.textContent = 'Все режимы';
  els.modesCount.textContent = String(list.length);
  renderModeGrid(els.modesGrid, list);
  setEmpty(
    list.length === 0,
    'Режимов пока нет',
    'Список появится, когда в репозитории будет modes.json'
  );
  updateSidebarCounts();
}

function renderLibrary() {
  if (!els.libraryView) return;
  const list = sortModes(
    S.modes.filter((mode) => inLibrary(mode) && matchesSearch(mode))
  );
  els.libraryCount.textContent = String(list.length);
  renderModeGrid(els.libraryGrid, list, true);
  const showEmpty = list.length === 0;
  els.libraryEmpty.hidden = !showEmpty;
  if (showEmpty) {
    els.libraryEmptyTitle.textContent = S.search ? 'Ничего не найдено' : 'Библиотека пуста';
    els.libraryEmptySub.textContent = S.search
      ? 'Попробуй изменить запрос'
      : 'Добавь режим в библиотеку из магазина, и он появится здесь';
  }
  updateSidebarCounts();
}

function renderFeatured(mode) {
  if (!mode) {
    els.featured.hidden = true;
    return;
  }
  els.featured.hidden = false;
  els.featuredImg.src = imageSrc(mode);
  wireImage(els.featuredImg, mode);
  els.featuredTitle.textContent = mode.title || 'Без названия';
  els.featuredDesc.textContent = modeDescription(mode);
  els.featuredTags.innerHTML = modeTags(mode, 4)
    .map((t) => `<span>${escapeHtml(t)}</span>`)
    .join('');
  const st = cardStatus(mode);
  els.featuredStatus.textContent = st.text;
}

function renderModeGrid(container, list, withMenu = false) {
  container.innerHTML = list.map((mode) => modeCardHtml(mode, withMenu)).join('');
  container.onclick = (e) => {
    const menuBtn = e.target.closest('.card-menu-btn');
    if (menuBtn) {
      e.stopPropagation();
      const menu = menuBtn.closest('.mode-card').querySelector('.card-menu');
      if (!menu) return;
      const wasHidden = menu.hidden;
      closeCardMenus();
      menu.hidden = !wasHidden;
      return;
    }
    const action = e.target.closest('[data-card-action]');
    if (action) {
      e.stopPropagation();
      const card = action.closest('.mode-card');
      if (card && card.dataset.id) handleCardAction(card.dataset.id, action.dataset.cardAction);
      return;
    }
    closeCardMenus();
    const card = e.target.closest('.mode-card');
    if (card && card.dataset.id) openDetail(card.dataset.id);
  };
  container.querySelectorAll('img[data-fallback]').forEach((img) => {
    const card = img.closest('.mode-card');
    wireImage(img, card ? modeById(card.dataset.id) : null);
  });
}

function closeCardMenus() {
  document.querySelectorAll('.card-menu:not([hidden])').forEach((menu) => {
    menu.hidden = true;
  });
}

async function handleCardAction(modeId, action) {
  closeCardMenus();
  const mode = modeById(modeId);
  if (!mode) return;

  if (action === 'folder') {
    try {
      await api.openModeFolder(modeId);
    } catch (err) {
      toast('Не удалось открыть папку: ' + err, 'error');
    }
    return;
  }

  if (action === 'hide') {
    try {
      const res = await api.setLibrary(modeId, false);
      if (!res || !res.ok) throw new Error((res && res.error) || 'Ошибка');
      const st = S.state[modeId] || { installed: false, version: null, installedAt: 0 };
      S.state[modeId] = { ...st, library: false, hidden: true };
      renderStore();
      renderLibrary();
      if (S.view === 'detail' && S.selectedId === modeId) renderDetail();
      toast('Режим убран из библиотеки', 'ok');
    } catch (err) {
      toast('Не удалось убрать режим: ' + err, 'error');
    }
    return;
  }

  if (action === 'uninstall') {
    if (!confirm(`Удалить режим «${mode.title}» с устройства? Файлы сборки будут удалены.`)) return;
    try {
      const res = await api.uninstall(modeId);
      if (!res || !res.ok) throw new Error((res && res.error) || 'Ошибка');
      const st = S.state[modeId] || { library: true, hidden: false };
      S.state[modeId] = { ...st, installed: false, version: null, installedAt: 0 };
      renderStore();
      renderLibrary();
      if (S.view === 'detail' && S.selectedId === modeId) renderDetail();
      toast('Режим удалён с устройства', 'ok');
    } catch (err) {
      toast('Не удалось удалить режим: ' + err, 'error');
    }
  }
}

function modeCardHtml(mode, withMenu = false) {
  const st = cardStatus(mode);
  const installed = isInstalled(mode);
  const tags = modeTags(mode, 3)
    .map((t) => `<span>${escapeHtml(t)}</span>`)
    .join('');
  return `
    <article class="mode-card ${installed ? 'installed' : ''}" data-id="${escapeHtml(mode.id)}">
      <div class="card-media">
        <img src="${escapeHtml(imageSrc(mode))}" alt="" loading="lazy" data-fallback>
        <div class="card-badge ${st.cls}">${st.text}</div>
        ${withMenu ? `
          <button class="card-menu-btn" type="button" title="Действия" aria-label="Действия">⋮</button>
          <div class="card-menu" hidden>
            ${installed ? '<button type="button" data-card-action="folder">Открыть папку</button>' : ''}
            <button type="button" data-card-action="hide">Удалить из библиотеки</button>
            ${installed ? '<button type="button" data-card-action="uninstall" class="danger">Удалить с устройства</button>' : ''}
          </div>` : ''}
      </div>
      <div class="card-body">
        <div class="card-title">${escapeHtml(mode.title || 'Без названия')}</div>
        <div class="card-sub">${escapeHtml(cardSub(mode))}</div>
        <div class="card-desc">${escapeHtml(modeDescription(mode))}</div>
        ${tags ? `<div class="tags">${tags}</div>` : ''}
        <div class="card-foot">
          <span class="card-size">${escapeHtml(mode.size || '—')}</span>
          <button class="btn btn-small btn-ghost" type="button" aria-label="Подробнее: ${escapeHtml(mode.title || 'режим')}">Подробнее</button>
        </div>
      </div>
    </article>`;
}

function setEmpty(show, title, sub) {
  els.emptyState.hidden = !show;
  if (show) {
    els.emptyTitle.textContent = title;
    els.emptySub.textContent = sub;
  }
}

function accentColor(mode) {
  const raw = String((mode && mode.accent) || '').trim();
  return /^#[0-9a-f]{6}$/i.test(raw) ? raw : DEFAULT_ACCENT;
}

function formatBytes(bytes) {
  const value = Number(bytes || 0);
  if (value >= 1024 * 1024 * 1024) return `${(value / 1024 / 1024 / 1024).toFixed(1)} ГБ`;
  if (value >= 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} МБ`;
  if (value >= 1024) return `${(value / 1024).toFixed(0)} КБ`;
  return `${value} Б`;
}

function tagPill(tag) {
  const meta = UPDATE_TAGS[tag] || { label: tag, cls: 'tech' };
  return `<span class="tag-pill ${escapeHtml(meta.cls)}">${escapeHtml(meta.label)}</span>`;
}

function formatUpdateDate(value) {
  return String(value || '').replace(/^(\d{4})-(\d{2})-(\d{2})$/, '$3.$2.$1');
}

function latestUpdateEntry(mode) {
  if (Array.isArray(mode.history) && mode.history.length) {
    return mode.history[0];
  }
  const parsed = parseChangelog(mode)[0];
  if (!parsed) return null;
  const match = String(parsed.title || '').match(/^v?(\d+\.\d+(?:\.\d+)?)/i);
  return {
    version: match ? match[1] : parsed.title,
    title: '',
    tags: [],
    date: '',
    changes: parsed.lines
  };
}

function renderSpotlight(mode) {
  const latest = latestUpdateEntry(mode);
  if (!latest) {
    els.detailSpotlight.hidden = true;
    return;
  }

  els.detailSpotlight.hidden = false;
  const version = latest.version ? `v${latest.version}` : '';
  const title = latest.title || '';
  els.spotlightTitle.textContent = [version, title].filter(Boolean).join(' — ') || 'Обновление режима';
  els.spotlightTag.innerHTML = latest.tags && latest.tags.length ? tagPill(latest.tags[0]) : '';
  els.spotlightDate.textContent = latest.date ? formatUpdateDate(latest.date) : '';
}

function fallbackArt(mode) {
  const key = [mode.id, mode.title, mode.accent].join('|');
  if (ART_CACHE.has(key)) return ART_CACHE.get(key);

  const accent = accentColor(mode);
  const words = String(mode.title || 'TM')
    .split(/\s+/)
    .map((word) => (word.match(/[A-Za-zА-Яа-я0-9]/) || [''])[0])
    .filter(Boolean);
  const initials = (words.slice(0, 2).join('') || 'T').toUpperCase();

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 450">
    <defs>
      <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#14141b"/>
        <stop offset="1" stop-color="${accent}" stop-opacity="0.5"/>
      </linearGradient>
      <radialGradient id="glow" cx="0.76" cy="0.34" r="0.7">
        <stop offset="0" stop-color="${accent}" stop-opacity="0.5"/>
        <stop offset="1" stop-color="${accent}" stop-opacity="0"/>
      </radialGradient>
    </defs>
    <rect width="800" height="450" fill="url(#bg)"/>
    <rect width="800" height="450" fill="url(#glow)"/>
    <g stroke="#ffffff" stroke-opacity="0.07" stroke-width="1">
      <path d="M0 300 H800 M0 350 H800 M0 400 H800"/>
      <path d="M520 0 V450 M600 0 V450 M680 0 V450"/>
    </g>
    <text x="744" y="330" text-anchor="end" font-family="Segoe UI, Arial, sans-serif" font-size="200" font-weight="800" fill="#ffffff" fill-opacity="0.9">${initials}</text>
    <text x="744" y="386" text-anchor="end" font-family="Segoe UI, Arial, sans-serif" font-size="18" font-weight="700" letter-spacing="7" fill="#ffffff" fill-opacity="0.55">TM MODE</text>
  </svg>`;

  const url = 'data:image/svg+xml;utf8,' + encodeURIComponent(svg);
  ART_CACHE.set(key, url);
  return url;
}

function imageSrc(mode) {
  return mode.imageUrl || mode.image || fallbackArt(mode);
}

function wireImage(img, mode) {
  if (!img) return;
  img.onerror = () => {
    img.onerror = null;
    img.src = fallbackArt(mode || { id: 'fallback', title: 'TM' });
  };
}

// ---------------------------------------------------------------- detail

function bindDetail() {
  els.detailBack.onclick = () => showStore();
  els.sideFolder.onclick = () => api.openGameFolder().catch(() => {});
  els.spotlightLink.onclick = () => {
    const section = els.detailChangelogSection;
    if (!section) return;
    const first = section.querySelector('details.changelog-item');
    if (first) first.open = true;
    section.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };
  els.changelogToggle.onclick = () => {
    const items = Array.from(els.detailChangelog.querySelectorAll('details.changelog-item'));
    const expand = items.some((item) => !item.open);
    items.forEach((item) => { item.open = expand; });
    updateChangelogToggle();
  };
  els.detailChangelog.addEventListener('toggle', updateChangelogToggle, true);
  els.detailAction.onclick = async () => {
    const mode = modeById(S.selectedId);
    if (!mode) return;
    if (S.busy && S.busy.modeId === mode.id) {
      await api.cancel();
      S.status = 'Отмена…';
      updateDetailAction();
      return;
    }
    if (S.busy) {
      toast('Дождитесь завершения текущей операции', 'warn');
      return;
    }
    if (S.running === mode.id) return;
    const action = computeAction(mode);
    if (action === 'running') return;
    if (action === 'add') {
      try {
        const res = await api.setLibrary(mode.id, true);
        if (!res || !res.ok) throw new Error((res && res.error) || 'Ошибка');
        const st = S.state[mode.id] || { installed: false, version: null, installedAt: 0 };
        S.state[mode.id] = { ...st, library: true, hidden: false };
        renderStore();
        renderLibrary();
        renderDetail();
        toast('Режим добавлен в библиотеку', 'ok');
      } catch (err) {
        toast('Не удалось добавить режим: ' + err, 'error');
      }
      return;
    }
    if (!S.current) {
      openModal('modal-account');
      toast('Сначала войдите в аккаунт Ely.by', 'warn');
      return;
    }

    S.busy = { modeId: mode.id, action };
    S.progress = 0;
    S.status = action === 'play' ? 'Подготовка к запуску…' : 'Загрузка файлов…';
    updateDetailAction();

    let res;
    try {
      res = action === 'play' ? await api.play(mode.id) : await api.install(mode.id);
    } catch (err) {
      res = { ok: false, error: String(err) };
    }
    if (!res || !res.ok) {
      S.busy = null;
      S.status = 'Ошибка';
      updateDetailAction();
      toast((res && res.error) || 'Не удалось запустить операцию', 'error');
    }
  };
}

function openDetail(id) {
  const mode = modeById(id);
  if (!mode) return;
  S.detailFrom = S.section || 'store';
  S.selectedId = id;
  els.storeView.hidden = true;
  els.libraryView.hidden = true;
  els.detailView.hidden = false;
  S.view = 'detail';
  els.detailBackLabel.textContent = S.detailFrom === 'library' ? 'Назад в библиотеку' : 'Назад в магазин';
  renderDetail();
  $('app').scrollTop = 0;
}

function showStore() {
  S.selectedId = null;
  const section = S.detailFrom || S.section || 'store';
  S.detailFrom = null;
  showSection(section);
}

function modeById(id) {
  return S.modes.find((m) => m.id === id) || null;
}

function computeAction(mode) {
  if (S.running === mode.id) return 'running';
  if (!inLibrary(mode)) return 'add';
  if (isUpdate(mode)) return 'update';
  if (isInstalled(mode)) return 'play';
  return 'install';
}

function actionLabel(action) {
  return {
    add: 'Добавить в библиотеку',
    install: 'Установить',
    update: 'Обновить',
    play: 'Играть',
    running: 'В игре'
  }[action] || 'Установить';
}

function defaultStatus(mode) {
  if (S.running === mode.id) return 'Игра запущена';
  if (!inLibrary(mode)) return 'Режим не добавлен в библиотеку';
  if (isUpdate(mode)) return 'Доступно обновление';
  if (isInstalled(mode)) return 'Готов к запуску';
  return 'Файлы ещё не скачаны';
}

function isChangelogHeading(line) {
  const text = String(line || '').trim();
  if (!text) return false;
  return /^(?:\[?v?\d+\.\d+(?:\.\d+)?(?:[-+][\w.]+)?\]?)(?:\s|$|[-—:])/i.test(text)
    || /^(?:версия|обновление)\s+\d/i.test(text)
    || /^\d{1,2}[./]\d{1,2}[./]\d{2,4}/.test(text);
}

function cleanChangelogLine(line) {
  return String(line || '').replace(/^[-—–•*]\s*/, '').trim();
}

function parseChangelog(mode) {
  const raw = (mode.changelog || '').trim();
  if (!raw) return [];

  const lines = raw.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const entries = [];
  let current = null;

  for (const line of lines) {
    if (isChangelogHeading(line)) {
      const title = line
        .replace(/^\[|\]$/g, '')
        .replace(/^(?:версия|обновление)\s+/i, '')
        .trim();
      current = { title, lines: [] };
      entries.push(current);
      continue;
    }

    if (!current) {
      current = {
        title: mode.version ? `Версия ${mode.version}` : 'Текущая версия',
        lines: []
      };
      entries.push(current);
    }

    const cleaned = cleanChangelogLine(line);
    if (cleaned) current.lines.push(cleaned);
  }

  return entries;
}

function changelogModel(mode) {
  if (Array.isArray(mode.history) && mode.history.length) {
    return mode.history.map((entry) => ({
      version: entry.version || '',
      title: entry.title || '',
      tags: Array.isArray(entry.tags) ? entry.tags : [],
      date: entry.date || '',
      lines: Array.isArray(entry.changes) ? entry.changes : []
    }));
  }
  return parseChangelog(mode).map((entry) => ({
    version: '',
    title: entry.title,
    tags: [],
    date: '',
    lines: entry.lines
  }));
}

function renderChangelog(mode) {
  const entries = changelogModel(mode);

  if (entries.length === 0) {
    els.detailChangelog.innerHTML = '<div class="changelog-empty">Пока нет записей об обновлениях.</div>';
    els.changelogToggle.hidden = true;
    return;
  }

  els.detailChangelog.innerHTML = entries.map((entry, index) => {
    const version = entry.version ? `v${entry.version}` : entry.title;
    const name = entry.version ? entry.title : '';
    return `
      <details class="changelog-item"${index === 0 ? ' open' : ''}>
        <summary>
          <span class="changelog-version">${escapeHtml(version)}</span>
          ${name ? `<span class="changelog-title">${escapeHtml(name)}</span>` : ''}
          ${entry.tags.length ? tagPill(entry.tags[0]) : ''}
          ${entry.date ? `<span class="changelog-date">${escapeHtml(formatUpdateDate(entry.date))}</span>` : ''}
          <svg class="changelog-chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 9 L12 15 L18 9"/></svg>
        </summary>
        <div class="changelog-body">
          ${entry.lines.length
            ? `<ul>${entry.lines.map((line) => `<li>${escapeHtml(line)}</li>`).join('')}</ul>`
            : '<p>Изменения не указаны.</p>'}
        </div>
      </details>`;
  }).join('');

  els.detailChangelog.classList.toggle('timeline', entries.length > 0);
  els.changelogToggle.hidden = entries.length < 2;
  updateChangelogToggle();
}

function updateChangelogToggle() {
  if (!els.changelogToggle) return;
  const items = Array.from(els.detailChangelog.querySelectorAll('details.changelog-item'));
  if (items.length === 0) {
    els.changelogToggle.hidden = true;
    return;
  }
  const allOpen = items.every((item) => item.open);
  els.changelogToggle.textContent = allOpen ? 'Свернуть всё' : 'Развернуть всё';
  els.changelogToggle.setAttribute('aria-expanded', allOpen ? 'true' : 'false');
}

function renderDetail() {
  const mode = modeById(S.selectedId);
  if (!mode) {
    showStore();
    return;
  }

  els.detailImg.src = imageSrc(mode);
  wireImage(els.detailImg, mode);
  els.detailTitle.textContent = mode.title || 'Без названия';
  els.detailSub.textContent = `${modeSub(mode)} · ${mode.size || '—'}`;
  els.detailTags.innerHTML = (Array.isArray(mode.tags) ? mode.tags : [])
    .filter(Boolean)
    .map((t) => `<span>${escapeHtml(t)}</span>`)
    .join('');

  const latest = latestUpdateEntry(mode);
  const updated = latest && latest.date ? formatUpdateDate(latest.date) : '—';
  els.detailMeta.innerHTML = [
    ['Версия', 'v' + (mode.version || '—')],
    ['Minecraft', mode.mcVersion || '—'],
    ['Загрузчик', loaderLabel(mode)],
    ['Размер', mode.size || '—'],
    ['Обновлено', updated]
  ].map(([label, value]) => `
    <div class="meta-item">
      <span class="meta-label">${label}</span>
      <span class="meta-value">${escapeHtml(value)}</span>
    </div>`).join('');

  els.detailDesc.textContent = modeDescription(mode);
  renderChangelog(mode);
  els.reqSize.textContent = mode.size || '—';
  renderSpotlight(mode);

  const others = sortByTitle(S.modes.filter((m) => m.id !== mode.id)).slice(0, 3);
  els.moreSection.hidden = others.length === 0;
  renderModeGrid(els.moreGrid, others);

  updateDetailAction();
}

function updateDetailAction() {
  if (S.view !== 'detail') return;
  const mode = modeById(S.selectedId);
  if (!mode) return;

  const busyHere = !!(S.busy && S.busy.modeId === mode.id);
  if (busyHere) {
    els.detailAction.textContent = 'Отмена';
    els.detailAction.className = 'btn btn-danger btn-big btn-block';
    els.detailAction.disabled = false;
    els.detailStatus.textContent = S.status || 'Подготовка…';
    els.detailProgress.hidden = false;
    els.detailProgressBar.style.width = Math.max(0, Math.min(100, S.progress * 100)) + '%';
    return;
  }

  const action = computeAction(mode);
  els.detailAction.textContent = actionLabel(action);
  els.detailAction.className = 'btn btn-primary btn-big btn-block';
  els.detailAction.disabled = action === 'running';
  els.detailStatus.textContent = defaultStatus(mode);
  els.detailProgress.hidden = true;
  els.detailProgressBar.style.width = '0%';
}

// ---------------------------------------------------------------- события игры

function onGameEvent(ev) {
  if (!ev || !ev.modeId) return;
  const mode = modeById(ev.modeId);
  const isCurrent = S.selectedId === ev.modeId;

  switch (ev.type) {
    case 'progress': {
      const p = ev.payload && ev.payload.total
        ? ev.payload.task / ev.payload.total
        : 0;
      S.progress = Math.max(0, Math.min(1, p));
      S.status = `Загрузка… ${Math.round(S.progress * 100)}%`;
      if (isCurrent) updateDetailAction();
      break;
    }
    case 'status': {
      S.status = String(ev.payload || '');
      if (isCurrent) updateDetailAction();
      break;
    }
    case 'installed': {
      S.busy = null;
      S.progress = 1;
      S.status = 'Установлено';
      if (mode) {
        const st = S.state[mode.id] || {};
        S.state[mode.id] = { ...st, installed: true, version: mode.version, installedAt: Date.now(), library: true, hidden: false };
      }
      toast('Режим установлен', 'ok');
      renderStore();
      renderLibrary();
      if (S.view === 'detail' && isCurrent) renderDetail();
      break;
    }
    case 'launched': {
      S.busy = null;
      S.running = ev.modeId;
      S.progress = 1;
      S.status = 'Игра запущена';
      toast('Игра запущена. Удачной катки!', 'ok');
      renderStore();
      renderLibrary();
      if (S.view === 'detail' && isCurrent) renderDetail();
      if (S.settings && S.settings.minimizeOnLaunch !== false) {
        api.minimize().catch(() => {});
      }
      break;
    }
    case 'exit': {
      S.running = null;
      S.busy = null;
      S.progress = 0;
      S.status = ev.payload === 0 ? 'Игра закрыта' : `Игра завершилась (код ${ev.payload})`;
      renderStore();
      renderLibrary();
      if (S.view === 'detail' && isCurrent) renderDetail();
      break;
    }
    case 'cancelled': {
      S.busy = null;
      S.progress = 0;
      S.status = 'Отменено';
      toast('Операция отменена', 'warn');
      renderStore();
      renderLibrary();
      if (S.view === 'detail' && isCurrent) renderDetail();
      break;
    }
    case 'error': {
      S.busy = null;
      S.progress = 0;
      if (S.running === ev.modeId) S.running = null;
      S.status = 'Ошибка';
      toast(String(ev.payload || 'Ошибка'), 'error');
      renderStore();
      renderLibrary();
      if (S.view === 'detail' && isCurrent) renderDetail();
      break;
    }
    default:
      break;
  }
}

// ---------------------------------------------------------------- обновление лаунчера

function bindUpdateModal() {
  const close = () => {
    $('modal-update').hidden = true;
  };
  $('btn-update-later').onclick = close;
  $('btn-update-later-x').onclick = close;
  els.updateChip.onclick = () => {
    if (!S.updateInfo) return;
    els.updateCurrent.textContent = `v${S.updateInfo.current}`;
    els.updateLatest.textContent = `v${S.updateInfo.latest}`;
    els.updateNotes.textContent = (S.updateInfo.notes || '').trim() || 'Список изменений появится позже.';
    els.btnUpdateDownload.textContent = S.updateInfo.installerUrl
      ? 'Скачать обновление'
      : 'Открыть релиз';
    $('modal-update').hidden = false;
  };
  els.btnUpdateDownload.onclick = async () => {
    const info = S.updateInfo;
    if (!info) return;
    if (!info.installerUrl) {
      api.openUrl(info.url).catch((err) => toast(String(err), 'error'));
      return;
    }
    els.btnUpdateDownload.disabled = true;
    els.btnUpdateDownload.textContent = 'Скачиваю…';
    try {
      await api.downloadUpdate(info.installerUrl);
      close();
      toast('Установщик открыт. Заверши установку обновления.', 'ok');
    } catch (err) {
      toast(String(err), 'error');
    } finally {
      els.btnUpdateDownload.disabled = false;
      els.btnUpdateDownload.textContent = 'Скачать обновление';
    }
  };
}

async function checkLauncherUpdate() {
  if (!api.checkUpdate) return null;
  try {
    const info = await api.checkUpdate();
    if (!info || !info.available) {
      els.updateChip.hidden = true;
      return null;
    }
    S.updateInfo = info;
    return info;
  } catch (_) {
    els.updateChip.hidden = true;
    return null;
  }
}

async function finishStartup() {
  try {
    if (api.windowReady) await api.windowReady();
  } catch (_) {
    // Окно уже могло быть закрыто.
  }
  finishSplash();
}

async function handleStartupUpdate() {
  const info = await checkLauncherUpdate();
  if (info && info.installerUrl) {
    startAutoUpdate(info);
    return;
  }
  if (info) {
    els.updateChipText.textContent = `Обновление ${info.latest}`;
    els.updateChip.hidden = false;
  }
  await finishStartup();
}

function showSplashUpdate(info) {
  if (els.splash) els.splash.classList.add('updating');
  if (els.splashNormal) els.splashNormal.hidden = true;
  if (els.splashUpdate) {
    els.splashUpdate.classList.remove('installing');
    els.splashUpdate.hidden = false;
  }
  if (els.splashUpdateVersion) els.splashUpdateVersion.textContent = `v${info.latest}`;
  if (els.splashUpdateRing) els.splashUpdateRing.style.strokeDashoffset = '402.12';
  if (els.splashUpdateBar) els.splashUpdateBar.style.width = '0%';
  if (els.splashUpdatePercent) els.splashUpdatePercent.textContent = '0%';
  if (els.splashUpdateStatus) {
    els.splashUpdateStatus.textContent = `Скачиваю обновление ${info.latest}…`;
  }
}

function setSplashProgress(percent, status) {
  const value = Math.max(0, Math.min(100, Math.round(percent || 0)));
  if (els.splashUpdateBar) els.splashUpdateBar.style.width = `${value}%`;
  if (els.splashUpdatePercent) els.splashUpdatePercent.textContent = `${value}%`;
  if (els.splashUpdateRing) els.splashUpdateRing.style.strokeDashoffset = `${(402.12 * (1 - value / 100)).toFixed(2)}`;
  if (status && els.splashUpdateStatus) els.splashUpdateStatus.textContent = status;
}

function hideSplashUpdate() {
  if (els.splash) els.splash.classList.remove('updating');
  if (els.splashUpdate) {
    els.splashUpdate.hidden = true;
    els.splashUpdate.classList.remove('installing');
  }
  if (els.splashNormal) els.splashNormal.hidden = false;
}

async function startAutoUpdate(info) {
  if (S.autoUpdating) return;
  S.autoUpdating = true;
  els.updateChip.hidden = true;
  showSplashUpdate(info);

  let unlisten = null;
  try {
    if (api.onUpdateProgress) {
      unlisten = await api.onUpdateProgress((payload) => {
        const total = Number(payload && payload.total) || 0;
        const downloaded = Number(payload && payload.downloaded) || 0;
        const percent = total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
        const status = total > 0
          ? `Скачивание… ${formatBytes(downloaded)} из ${formatBytes(total)}`
          : 'Скачивание обновления…';
        setSplashProgress(percent, status);
      });
    }

    await api.installUpdate(info.installerUrl);
    if (els.splashUpdate) els.splashUpdate.classList.add('installing');
    setSplashProgress(100, 'Устанавливаю обновление. Лаунчер сейчас закроется…');
    setTimeout(() => {
      api.close().catch(() => {});
    }, 1500);
  } catch (err) {
    S.autoUpdating = false;
    hideSplashUpdate();
    els.updateChipText.textContent = `Обновление ${info.latest}`;
    els.updateChip.hidden = false;
    toast('Автообновление не удалось: ' + err, 'error');
    await finishStartup();
  } finally {
    if (typeof unlisten === 'function') unlisten();
  }
}

// ---------------------------------------------------------------- аккаунт

function avatarSkin(acc) {
  if (!acc || acc.type !== 'ely' || !acc.name) return '';
  return `url("https://skinsystem.ely.by/skins/${encodeURIComponent(acc.name)}.png")`;
}

function avatarLetter(acc) {
  if (!acc || !acc.name) return 'Г';
  return acc.name[0].toUpperCase();
}

function applyAvatar(el, acc) {
  if (!el) return;
  el.classList.remove('loaded');
  el.style.backgroundImage = '';
  el.dataset.avatarUrl = '';

  if (acc && acc.type === 'ely' && acc.name) {
    const url = `https://skinsystem.ely.by/skins/${encodeURIComponent(acc.name)}.png`;
    el.textContent = avatarLetter(acc);
    el.dataset.avatarUrl = url;
    const img = new Image();
    img.onload = () => {
      if (el.dataset.avatarUrl !== url) return;
      const hasHat = img.naturalWidth >= 64 && img.naturalHeight >= 64;
      el.style.backgroundImage = hasHat
        ? `${avatarSkin(acc)}, ${avatarSkin(acc)}`
        : avatarSkin(acc);
      el.textContent = '';
      el.classList.add('loaded');
    };
    img.onerror = () => {
      if (el.dataset.avatarUrl !== url) return;
      el.style.backgroundImage = '';
      el.textContent = avatarLetter(acc);
    };
    img.src = url;
  } else {
    el.textContent = avatarLetter(acc);
  }
}

function renderAccountChip() {
  els.accName.textContent = S.current ? S.current.name : 'Гость';
  applyAvatar(els.accAvatar, S.current);
}

function bindAccountModal() {
  els.accChip.onclick = () => openModal('modal-account');
  $('btn-ely-open').onclick = () => openElyLogin();
  $('btn-ely-start').onclick = () => startElyLogin();
  $('btn-ely-window').onclick = () => {
    if (!S.elyLogin || !S.elyLogin.verificationUriComplete) return;
    api.openElyWindow(S.elyLogin.verificationUriComplete).catch((err) => {
      $('ely-status-text').textContent = 'Не удалось открыть окно: ' + err;
    });
  };
  $('btn-ely-cancel').onclick = () => cancelElyLogin();
  $('ely-register').onclick = () => api.openUrl('https://account.ely.by/register').catch(() => {});
}

function openElyLogin() {
  openModal('modal-ely-login');
  resetElyLoginUI();
  startElyLogin();
}

async function startElyLogin() {
  if (S.elyLogin && S.elyLogin.polling) return;
  resetElyLoginUI();

  const status = $('ely-status-text');
  status.textContent = 'Запускаем вход…';
  $('btn-ely-start').hidden = true;
  $('btn-ely-cancel').hidden = false;

  let data;
  try {
    data = await api.deviceStart();
  } catch (err) {
    showElyError(String(err));
    return;
  }

  S.elyLogin = {
    deviceCode: data.deviceCode,
    verificationUriComplete: data.verificationUriComplete,
    interval: Math.max(5, Number(data.interval) || 5),
    polling: true,
    timer: null
  };

  status.textContent = 'Подтверди вход в окне Ely.by…';
  $('btn-ely-window').hidden = false;
  $('btn-ely-cancel').hidden = false;

  try {
    await api.openElyWindow(data.verificationUriComplete);
  } catch (err) {
    status.textContent = 'Не удалось открыть окно. Нажми «Открыть окно Ely.by».';
  }
  scheduleElyPoll();
}

function scheduleElyPoll() {
  if (!S.elyLogin) return;
  if (S.elyLogin.timer) clearTimeout(S.elyLogin.timer);
  S.elyLogin.timer = setTimeout(pollElyLogin, S.elyLogin.interval * 1000);
}

async function pollElyLogin() {
  if (!S.elyLogin || !S.elyLogin.polling) return;
  let res;
  try {
    res = await api.devicePoll(S.elyLogin.deviceCode);
  } catch (err) {
    showElyError(String(err));
    return;
  }
  if (!S.elyLogin) return;
  if (res && res.status === 'pending') {
    scheduleElyPoll();
    return;
  }
  if (res && res.status === 'slow_down') {
    S.elyLogin.interval += 5;
    scheduleElyPoll();
    return;
  }
  if (res && res.status === 'ok' && res.account) {
    await finishElyLogin(res.account);
    return;
  }
  showElyError('Ely.by вернул неожиданный ответ');
}

async function finishElyLogin(account) {
  stopElyLogin();
  S.current = account;
  try {
    S.accounts = (await api.getAccounts()) || [];
  } catch (_) {
    S.accounts = [];
  }
  closeElyLogin();
  renderAccountChip();
  renderAccountModal();
  toast(`Добро пожаловать, ${account.name}!`, 'ok');
}

function showElyError(message) {
  if (S.elyLogin && S.elyLogin.timer) clearTimeout(S.elyLogin.timer);
  S.elyLogin = null;
  api.closeElyWindow().catch(() => {});
  const err = $('ely-login-error');
  err.textContent = message;
  err.hidden = false;
  $('ely-status-text').textContent = 'Вход не выполнен';
  const start = $('btn-ely-start');
  start.hidden = false;
  start.disabled = false;
  start.textContent = 'Повторить';
  $('btn-ely-window').hidden = true;
  $('btn-ely-cancel').hidden = true;
}

function cancelElyLogin() {
  stopElyLogin();
  resetElyLoginUI();
}

function stopElyLogin() {
  if (S.elyLogin && S.elyLogin.timer) clearTimeout(S.elyLogin.timer);
  S.elyLogin = null;
  api.closeElyWindow().catch(() => {});
}

function resetElyLoginUI() {
  const err = $('ely-login-error');
  if (err) {
    err.hidden = true;
    err.textContent = '';
  }
  const status = $('ely-status-text');
  if (status) status.textContent = 'Откроется окно Ely.by. Войди в аккаунт и подтверди вход.';
  const start = $('btn-ely-start');
  if (start) {
    start.hidden = false;
    start.disabled = false;
    start.textContent = 'Продолжить';
  }
  const win = $('btn-ely-window');
  if (win) win.hidden = true;
  const cancel = $('btn-ely-cancel');
  if (cancel) cancel.hidden = true;
}

function closeElyLogin() {
  stopElyLogin();
  const modal = $('modal-ely-login');
  if (modal) modal.hidden = true;
  resetElyLoginUI();
}

function renderAccountModal() {
  const cur = S.current;
  const box = $('acc-current');
  box.innerHTML = '';
  if (cur) {
    const av = document.createElement('span');
    av.className = 'avatar';
    applyAvatar(av, cur);
    const info = document.createElement('div');
    const sub = cur.type === 'ely'
      ? (cur.sessionError ? '<span class="warn">Сессия истекла — войдите заново</span>' : 'Аккаунт Ely.by')
      : 'Офлайн-аккаунт';
    info.innerHTML = `<div class="acc-name"></div><div class="acc-sub">${sub}</div>`;
    info.querySelector('.acc-name').textContent = cur.name;
    box.appendChild(av);
    box.appendChild(info);
  } else {
    box.innerHTML = '<div style="color:var(--muted);font-size:12.5px;">Аккаунтов нет — войди через Ely.by</div>';
  }
  renderAccountList();
  const label = $('ely-open-label');
  if (label) {
    label.textContent = cur && cur.type === 'ely' ? 'Сменить аккаунт Ely.by' : 'Войти через Ely.by';
  }
}

function renderAccountList() {
  const list = $('acc-list');
  list.innerHTML = '';
  if (!S.accounts.length) {
    list.innerHTML = '<div style="color:var(--muted);font-size:12px;">Пока пусто</div>';
    return;
  }
  for (const acc of S.accounts) {
    const row = document.createElement('div');
    row.className = 'acc-row' + (S.current && S.current.id === acc.id ? ' current' : '');

    const av = document.createElement('span');
    av.className = 'avatar';
    applyAvatar(av, acc);

    const name = document.createElement('span');
    name.className = 'acc-name';
    name.textContent = acc.name;

    const type = document.createElement('span');
    type.className = 'acc-type' + (acc.type === 'offline' ? ' offline' : '');
    type.textContent = acc.type === 'ely' ? 'ely.by' : 'офлайн';

    const pick = document.createElement('button');
    pick.textContent = S.current && S.current.id === acc.id ? 'Выбран' : 'Выбрать';
    pick.disabled = S.current && S.current.id === acc.id;
    pick.onclick = async () => {
      try {
        S.current = await api.selectAccount(acc.id);
        renderAccountChip();
        renderAccountModal();
        toast(`Аккаунт: ${acc.name}`, 'ok');
      } catch (ex) {
        toast('Не удалось выбрать аккаунт: ' + ex, 'error');
      }
    };

    const del = document.createElement('button');
    del.className = 'del';
    del.textContent = '✕';
    del.title = 'Удалить аккаунт';
    del.onclick = async () => {
      try {
        const res = await api.removeAccount(acc.id);
        S.accounts = res.accounts || [];
        S.current = res.current || null;
        renderAccountChip();
        renderAccountModal();
        toast('Аккаунт удалён', 'ok');
      } catch (ex) {
        toast('Не удалось удалить аккаунт: ' + ex, 'error');
      }
    };

    row.append(av, name, type, pick, del);
    list.appendChild(row);
  }
}

// ---------------------------------------------------------------- настройки

function bindSettings() {
  els.ram.addEventListener('input', () => {
    els.ramOut.textContent = `${els.ram.value} ГБ`;
  });
  $('btn-choose-dir').onclick = async () => {
    try {
      const dir = await api.chooseDir();
      if (dir) els.setDir.value = dir;
    } catch (ex) {
      toast('Не удалось выбрать папку: ' + ex, 'error');
    }
  };
  $('btn-open-dir').onclick = () => api.openGameFolder().catch(() => {});
  $('btn-open-data').onclick = () => api.openAppFolder().catch(() => {});
  document.querySelectorAll('.settings-tab').forEach((tab) => {
    tab.onclick = () => setSettingsTab(tab.dataset.settingsTab);
  });
  $('btn-save-settings').onclick = async () => {
    try {
      const s = await api.saveSettings({
        ramGb: Number(els.ram.value),
        dir: els.setDir.value.trim() || (S.settings && S.settings.dir) || '',
        javaPath: els.setJava.value.trim(),
        width: Number(els.setWidth.value) || 925,
        height: Number(els.setHeight.value) || 530,
        fullscreen: els.setFullscreen.checked,
        jvmFlags: els.setJvmFlags.checked,
        minimizeOnLaunch: els.setMinimizeOnLaunch.checked
      });
      S.settings = s;
      closeModals();
      toast('Настройки сохранены', 'ok');
    } catch (err) {
      toast('Не удалось сохранить настройки: ' + err, 'error');
    }
  };
}

function setSettingsTab(name) {
  const tab = name === 'launcher' ? 'launcher' : 'minecraft';
  document.querySelectorAll('.settings-tab').forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.settingsTab === tab);
  });
  $('settings-minecraft').hidden = tab !== 'minecraft';
  $('settings-launcher').hidden = tab !== 'launcher';
}

function fillSettingsForm() {
  const s = S.settings || {};
  els.ram.value = s.ramGb || 4;
  els.ramOut.textContent = `${s.ramGb || 4} ГБ`;
  els.setDir.value = s.dir || '';
  els.setJava.value = s.javaPath || '';
  els.setWidth.value = s.width || 925;
  els.setHeight.value = s.height || 530;
  els.setFullscreen.checked = !!s.fullscreen;
  els.setJvmFlags.checked = s.jvmFlags !== false;
  els.setMinimizeOnLaunch.checked = s.minimizeOnLaunch !== false;
  els.setDataDir.value = S.dataDir || '';
}

// ---------------------------------------------------------------- модалки

function openModal(id) {
  if (id === 'modal-settings') {
    fillSettingsForm();
    setSettingsTab('minecraft');
  }
  if (id === 'modal-account') renderAccountModal();
  if (id === 'modal-ely-login') {
    resetElyLoginUI();
  }
  $(id).hidden = false;
}

function closeModals() {
  document.querySelectorAll('.modal-overlay').forEach((m) => (m.hidden = true));
  closeElyLogin();
}

function closeTopModal() {
  const openModals = [...document.querySelectorAll('.modal-overlay:not([hidden])')];
  if (!openModals.length) return false;
  const top = openModals[openModals.length - 1];
  if (top.id === 'modal-ely-login') {
    closeElyLogin();
  } else {
    top.hidden = true;
  }
  return true;
}

function bindModals() {
  document.querySelectorAll('[data-close]').forEach((btn) => {
    btn.onclick = () => {
      if (btn.dataset.close === 'modal-ely-login') {
        closeElyLogin();
      } else {
        const target = $(btn.dataset.close);
        if (target) target.hidden = true;
      }
    };
  });
  document.querySelectorAll('.modal-overlay').forEach((ov) => {
    ov.addEventListener('mousedown', (e) => {
      if (e.target !== ov) return;
      if (ov.id === 'modal-ely-login') closeElyLogin();
      else ov.hidden = true;
    });
  });
  document.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape') return;
    if (closeTopModal()) {
      e.preventDefault();
      e.stopImmediatePropagation();
    }
  });
}

// ---------------------------------------------------------------- утилиты

function escapeHtml(s) {
  return String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
  })[c]);
}

let toastTimer = null;
function toast(msg, type = 'info') {
  els.toast.textContent = msg;
  els.toast.className = 'show ' + type;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (els.toast.className = ''), 3200);
}
