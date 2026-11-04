'use strict';

const api = window.tdev || mockApi();
const $ = (id) => document.getElementById(id);

const LOADER_LABELS = { vanilla: 'Vanilla', forge: 'Forge', fabric: 'Fabric' };

// Заглушка для режимов без арта (картинки живут только в репо)
const PLACEHOLDER =
  'data:image/svg+xml;utf8,' +
  encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 450">
      <defs>
        <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stop-color="#141430"/><stop offset="1" stop-color="#1c1440"/>
        </linearGradient>
        <radialGradient id="gl" cx="0.5" cy="0.4" r="0.6">
          <stop offset="0" stop-color="#7c5cff" stop-opacity="0.35"/><stop offset="1" stop-color="#7c5cff" stop-opacity="0"/>
        </radialGradient>
      </defs>
      <rect width="800" height="450" fill="url(#bg)"/>
      <rect width="800" height="450" fill="url(#gl)"/>
      <g stroke="#7c5cff" opacity="0.18">
        <line x1="400" y1="230" x2="120" y2="450"/><line x1="400" y1="230" x2="280" y2="450"/>
        <line x1="400" y1="230" x2="520" y2="450"/><line x1="400" y1="230" x2="680" y2="450"/>
        <line x1="0" y1="330" x2="800" y2="330"/><line x1="0" y1="380" x2="800" y2="380"/><line x1="0" y1="425" x2="800" y2="425"/>
      </g>
      <path d="M400 155 L460 190 V260 L400 295 L340 260 V190 Z" fill="none" stroke="#7c5cff" stroke-width="5" opacity="0.9"/>
      <path d="M400 190 L430 207 V242 L400 259 L370 242 V207 Z" fill="rgba(124,92,255,0.25)" stroke="#00e0ff" stroke-width="2.5" opacity="0.8"/>
    </svg>`
  );

const els = {
  cards: $('modes-cards'), count: $('modes-count'),
  detailEmpty: $('detail-empty'), detailBody: $('detail-body'), detailFooter: $('detail-footer'),
  detailImg: $('detail-img'), detailTitle: $('detail-title'), detailTags: $('detail-tags'),
  metaVersion: $('meta-version'), metaMc: $('meta-mc'), metaLoader: $('meta-loader'), metaSize: $('meta-size'), metaPlayers: $('meta-players'),
  detailDesc: $('detail-desc'), detailChangelog: $('detail-changelog'),
  status: $('action-status'), progressWrap: $('progress-wrap'), progressBar: $('progress-bar'),
  actionBtn: $('action-btn'),
  accAvatar: $('account-avatar'), accName: $('account-name'), accChip: $('account-chip'),
  toast: $('toast')
};

const S = {
  modes: [], state: {}, settings: null,
  accounts: [], current: null,
  selectedId: null,
  busy: null,      // { modeId, kind: 'install'|'play' }
  running: null,   // modeId запущенной игры
  progressTimer: null,
  pub: { imagePath: null, zipPath: null }
};

// ---------------------------------------------------------------- init

init();

async function init() {
  bindWindowButtons();
  bindModals();
  bindSettings();
  bindAccountModal();
  bindPublish();
  api.onGameEvent(onGameEvent);
  if (api.onPublishEvent) api.onPublishEvent(onPublishEvent);

  S.modes = (await api.getModes()) || [];
  S.state = (await api.getState()) || {};
  S.settings = await api.getSettings();
  S.accounts = (await api.getAccounts()) || [];
  S.current = await api.getCurrentAccount();

  renderAccountChip();
  renderModes();
  if (S.modes.length) selectMode(S.modes[0].id);

  if (api.refreshSession) {
    api.refreshSession().then((a) => {
      if (a) { S.current = a; renderAccountChip(); }
    });
  }

  // Превью/глубокая ссылка: ?open=settings|account
  const openParam = new URLSearchParams(location.search).get('open');
  if (openParam && $('modal-' + openParam)) openModal('modal-' + openParam);
}

// ---------------------------------------------------------------- окно

function bindWindowButtons() {
  $('btn-min').onclick = () => api.minimize();
  $('btn-settings').onclick = () => openModal('modal-settings');
  $('btn-close').onclick = () => api.close();
  $('titlebar').addEventListener('dblclick', (e) => {
    if (e.target.closest('button')) return;
    api.toggleMax();
  });
  $('folder-link').onclick = () => api.openGameFolder();
  if ($('site-link')) $('site-link').onclick = () => api.openUrl('https://tdev.gg');

  // незаметная кнопка обновления списка режимов
  const refreshBtn = $('btn-refresh');
  if (refreshBtn) {
    refreshBtn.onclick = async () => {
      if (refreshBtn.classList.contains('spinning')) return;
      refreshBtn.classList.add('spinning');
      try {
        S.modes = await api.refreshModes();
        S.state = await api.getState();
        renderModes();
        if (S.selectedId) {
          if (modeById(S.selectedId)) renderDetail(modeById(S.selectedId));
          else {
            S.selectedId = null;
            els.detailEmpty.hidden = false;
            els.detailBody.hidden = true;
            els.detailFooter.hidden = true;
          }
        }
        toast('Список режимов обновлён', 'ok');
      } finally {
        refreshBtn.classList.remove('spinning');
      }
    };
  }
}

// ---------------------------------------------------------------- режимы

function modeById(id) {
  return S.modes.find((m) => m.id === id);
}

function loaderSuffix(mode) {
  return mode.loader && mode.loader !== 'vanilla' ? ` • ${LOADER_LABELS[mode.loader] || mode.loader}` : '';
}

function cardStatus(mode) {
  if (S.running === mode.id) return { cls: 'ok', text: 'В игре' };
  const st = S.state[mode.id];
  if (!st || !st.installed) return { cls: 'new', text: 'Новый' };
  if (st.version !== mode.version) return { cls: 'update', text: 'Обновление' };
  return { cls: 'ok', text: 'Установлен' };
}

function renderModes() {
  els.count.textContent = String(S.modes.length);
  els.cards.innerHTML = '';

  if (!S.modes.length) {
    const empty = document.createElement('div');
    empty.className = 'modes-empty';
    empty.innerHTML = '<b>Режимов пока нет</b>Нажми <span class="plus">+</span> рядом с заголовком,<br>чтобы выложить первый режим в репозиторий';
    els.cards.appendChild(empty);
    return;
  }

  for (const m of S.modes) {
    const st = cardStatus(m);
    const el = document.createElement('div');
    el.className = 'mode-card' + (m.id === S.selectedId ? ' active' : '');
    el.innerHTML = `
      <img src="${m.imageUrl || m.image || PLACEHOLDER}" alt="" draggable="false">
      <div class="shade"></div>
      <div class="card-text">
        <div class="name"></div>
        <div class="sub">MC ${escapeHtml(m.mcVersion || '')}${loaderSuffix(m)}</div>
      </div>
      <div class="card-badge ${st.cls}">${st.text}</div>`;
    el.querySelector('.name').textContent = m.title;
    el.onclick = () => selectMode(m.id);
    els.cards.appendChild(el);
  }
}

function selectMode(id) {
  S.selectedId = id;
  const m = modeById(id);
  if (!m) return;
  document.querySelectorAll('.mode-card').forEach((el, i) => {
    el.classList.toggle('active', S.modes[i] && S.modes[i].id === id);
  });
  renderDetail(m);
}

function renderDetail(m) {
  els.detailEmpty.hidden = true;
  els.detailBody.hidden = false;
  els.detailFooter.hidden = false;

  els.detailImg.src = m.imageUrl || m.image || PLACEHOLDER;
  els.detailTitle.textContent = m.title;
  els.detailTags.innerHTML = '';  for (const t of m.tags || []) {
    const s = document.createElement('span');
    s.textContent = t;
    els.detailTags.appendChild(s);
  }
  els.metaVersion.textContent = 'v' + (m.version || '—');
  els.metaMc.textContent = m.mcVersion || '—';
  const loaderLabel = m.loader && m.loader !== 'vanilla'
    ? `${LOADER_LABELS[m.loader] || m.loader}${m.loaderVersion ? ' ' + m.loaderVersion : ''}`
    : 'Vanilla';
  els.metaLoader.textContent = loaderLabel;
  els.metaSize.textContent = m.size || '—';
  els.metaPlayers.textContent = m.players && m.players !== '—' ? `${m.players} online` : '—';
  els.detailDesc.textContent = m.description || 'Описание не заполнено';
  document.querySelector('#detail-body .changelog').style.display = m.changelog ? '' : 'none';
  els.detailChangelog.textContent = m.changelog;

  updateActionButton();
}

function computeAction(mode) {
  if (S.running === mode.id) return 'running';
  const st = S.state[mode.id];
  if (!st || !st.installed) return 'install';
  if (st.version !== mode.version) return 'update';
  return 'play';
}

const ACTION_LABEL = { install: 'УСТАНОВИТЬ', update: 'ОБНОВИТЬ', play: 'ИГРАТЬ', running: 'ИГРАЕТ…' };

function updateActionButton() {
  const m = modeById(S.selectedId);
  if (!m) return;
  const busyHere = S.busy && S.busy.modeId === m.id;
  if (busyHere) {
    // кнопка становится «Отменить» — защита от зависших загрузок
    els.actionBtn.disabled = false;
    els.actionBtn.textContent = 'ОТМЕНИТЬ';
    els.actionBtn.className = 'btn-cancel';
    return;
  }
  const action = computeAction(m);
  els.actionBtn.disabled = action === 'running';
  els.actionBtn.textContent = ACTION_LABEL[action];
  els.actionBtn.className = {
    install: '', update: 'btn-update', play: 'btn-play', running: 'btn-play'
  }[action];
  els.status.textContent = defaultStatus(m);
}

function defaultStatus(m) {
  const st = cardStatus(m);
  return st.text === 'В игре' ? 'Игра запущена' : st.cls === 'ok' ? 'Готов к запуску' : 'Файлы не скачаны';
}

els.actionBtn.onclick = async () => {
  // во время загрузки кнопка отменяет операцию
  if (S.busy) {
    await api.cancel();
    return;
  }
  const m = modeById(S.selectedId);
  if (!m) return;
  if (!S.current) {
    openModal('modal-account');
    toast('Сначала войдите в аккаунт Ely.by', 'warn');
    return;
  }
  const action = computeAction(m);
  if (action === 'running') return;
  const res = action === 'play' ? await api.play(m.id) : await api.install(m.id);
  if (!res || !res.ok) {
    toast((res && res.error) || 'Не удалось запустить операцию', 'error');
    return;
  }
  S.busy = { modeId: m.id, kind: action };
  setProgress(0);
  els.status.textContent = action === 'play' ? 'Подготовка к запуску…' : 'Загрузка файлов…';
  updateActionButton();
};

function setProgress(p) {
  els.progressWrap.hidden = false;
  els.progressBar.style.width = Math.max(0, Math.min(100, p)) + '%';
}

function hideProgressLater() {
  clearTimeout(S.progressTimer);
  S.progressTimer = setTimeout(() => {
    els.progressWrap.hidden = true;
    els.progressBar.style.width = '0%';
  }, 1400);
}

// ---------------------------------------------------------------- события игры

function onGameEvent(ev) {
  if (!ev || !ev.modeId) return;
  const isCurrent = ev.modeId === S.selectedId;

  switch (ev.type) {
    case 'progress': {
      const p = ev.payload && ev.payload.total ? (ev.payload.task / ev.payload.total) * 100 : 0;
      if (isCurrent) {
        setProgress(p);
        els.status.textContent = `Загрузка… ${Math.round(p)}%`;
      }
      break;
    }
    case 'installed': {
      const m = modeById(ev.modeId);
      S.state[ev.modeId] = { installed: true, version: m ? m.version : null };
      S.busy = null;
      if (isCurrent) {
        setProgress(100);
        els.status.textContent = 'Установлено ✔';
        hideProgressLater();
        updateActionButton();
      }
      toast('Режим установлен', 'ok');
      renderModes();
      break;
    }
    case 'launched': {
      S.busy = null;
      S.running = ev.modeId;
      if (isCurrent) {
        els.status.textContent = 'Игра запущена';
        setProgress(100);
        hideProgressLater();
        updateActionButton();
      }
      toast('Игра запущена. Удачной катки!', 'ok');
      renderModes();
      break;
    }
    case 'exit': {
      const wasRunning = S.running === ev.modeId;
      S.running = null;
      S.busy = null;
      if (isCurrent) {
        els.status.textContent = wasRunning ? 'Игра закрыта' : 'Процесс завершён';
        hideProgressLater();
        updateActionButton();
      }
      renderModes();
      break;
    }
    case 'error': {
      S.busy = null;
      if (isCurrent) {
        els.status.textContent = 'Ошибка';
        hideProgressLater();
        updateActionButton();
      }
      toast(String(ev.payload || 'Ошибка'), 'error');
      renderModes();
      break;
    }
    case 'status': {
      if (isCurrent) els.status.textContent = String(ev.payload || '');
      break;
    }
    case 'log':
    case 'download-status':
    default:
      break;
  }
}

// ---------------------------------------------------------------- модалки

function openModal(id) {
  if (id === 'modal-settings') fillSettingsForm();
  if (id === 'modal-account') renderAccountModal();
  if (id === 'modal-publish' && typeof fillPublishForm === 'function') fillPublishForm();
  $(id).hidden = false;
}
function closeModals() {
  document.querySelectorAll('.modal-overlay').forEach((m) => (m.hidden = true));
}

function bindModals() {
  document.querySelectorAll('[data-close]').forEach((btn) => {
    btn.onclick = closeModals;
  });
  document.querySelectorAll('.modal-overlay').forEach((ov) => {
    ov.addEventListener('mousedown', (e) => {
      if (e.target === ov) closeModals();
    });
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeModals();
  });
}

// ---------------------------------------------------------------- настройки

function fillSettingsForm() {
  const s = S.settings || {};
  $('set-ram').value = s.ramGb || 4;
  $('ram-out').textContent = `${s.ramGb || 4} ГБ`;
  $('set-dir').value = s.dir || '';
  $('set-java').value = s.javaPath || '';
  $('set-width').value = s.width || 925;
  $('set-height').value = s.height || 530;
  $('set-fullscreen').checked = !!s.fullscreen;
}

function bindSettings() {
  $('set-ram').addEventListener('input', () => {
    $('ram-out').textContent = `${$('set-ram').value} ГБ`;
  });
  $('btn-choose-dir').onclick = async () => {
    const dir = await api.chooseDir();
    if (dir) $('set-dir').value = dir;
  };
  $('btn-open-dir').onclick = () => api.openGameFolder();
  $('btn-save-settings').onclick = async () => {
    S.settings = await api.saveSettings({
      ramGb: Number($('set-ram').value),
      dir: $('set-dir').value.trim() || S.settings.dir,
      javaPath: $('set-java').value.trim(),
      width: Number($('set-width').value) || 925,
      height: Number($('set-height').value) || 530,
      fullscreen: $('set-fullscreen').checked
    });
    closeModals();
    toast('Настройки сохранены', 'ok');
  };
}

// ---------------------------------------------------------------- аккаунты

function avatarSkin(acc) {
  if (!acc) return '';
  if (acc.type === 'ely') {
    return `background-image:url('https://skinsystem.ely.by/skins/${encodeURIComponent(acc.name)}.png');background-size:256px auto;background-position:-32px -32px;`;
  }
  return '';
}

function avatarLetter(acc) {
  if (!acc) return 'Г';
  return acc.name ? acc.name[0].toUpperCase() : '?';
}

function renderAccountChip() {
  const acc = S.current;
  els.accName.textContent = acc ? acc.name : 'Гость';
  els.accAvatar.style.backgroundImage = acc ? avatarSkin(acc) : '';
  els.accAvatar.textContent = acc ? '' : avatarLetter(null);
}

function renderAccountModal() {
  const cur = S.current;
  const box = $('acc-current');
  box.innerHTML = '';
  if (cur) {
    const av = document.createElement('span');
    av.className = 'avatar';
    av.style.cssText = avatarSkin(cur);
    av.textContent = avatarLetter(cur);
    const info = document.createElement('div');
    const sub = cur.type === 'ely'
      ? (cur.sessionError ? '<span class="warn">Сессия истекла — войдите заново</span>' : 'Аккаунт Ely.by')
      : 'Офлайн-аккаунт';
    info.innerHTML = `<div class="acc-name"></div><div class="acc-sub">${sub}</div>`;
    info.querySelector('.acc-name').textContent = cur.name;
    box.appendChild(av);
    box.appendChild(info);
  } else {
    box.innerHTML = '<div style="color:var(--muted);font-size:13px;">Аккаунтов нет — войди через Ely.by или создай офлайн-профиль</div>';
  }
  renderAccountList();
  $('ely-error').hidden = true;
  $('off-error').hidden = true;
}

function renderAccountList() {
  const list = $('acc-list');
  list.innerHTML = '';
  if (!S.accounts.length) {
    list.innerHTML = '<div style="color:var(--muted);font-size:12.5px;">Пока пусто</div>';
    return;
  }
  for (const acc of S.accounts) {
    const row = document.createElement('div');
    row.className = 'acc-row' + (S.current && S.current.id === acc.id ? ' current' : '');
    const av = document.createElement('span');
    av.className = 'avatar';
    av.style.cssText = avatarSkin(acc);
    av.textContent = avatarLetter(acc);
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
      S.current = await api.selectAccount(acc.id);
      renderAccountChip();
      renderAccountModal();
      toast(`Аккаунт: ${acc.name}`, 'ok');
    };
    const del = document.createElement('button');
    del.className = 'del';
    del.textContent = '✕';
    del.title = 'Удалить аккаунт';
    del.onclick = async () => {
      const res = await api.removeAccount(acc.id);
      S.accounts = res.accounts;
      S.current = res.current;
      renderAccountChip();
      renderAccountModal();
    };
    row.appendChild(av);
    row.appendChild(name);
    row.appendChild(type);
    row.appendChild(pick);
    row.appendChild(del);
    list.appendChild(row);
  }
}

function bindAccountModal() {
  els.accChip.onclick = () => openModal('modal-account');

  document.querySelectorAll('.tab').forEach((tab) => {
    tab.onclick = () => {
      document.querySelectorAll('.tab').forEach((t) => t.classList.toggle('active', t === tab));
      $('form-ely').hidden = tab.dataset.tab !== 'ely';
      $('form-offline').hidden = tab.dataset.tab !== 'offline';
    };
  });

  $('ely-register').onclick = () => api.openUrl('https://account.ely.by/register');
  $('ely-site').onclick = () => api.openUrl('https://account.ely.by/auth/login');

  $('form-ely').onsubmit = async (e) => {
    e.preventDefault();
    const btn = $('btn-ely-login');
    const err = $('ely-error');
    err.hidden = true;
    btn.disabled = true;
    btn.textContent = 'ВХОД…';
    const res = await api.loginEly($('ely-user').value.trim(), $('ely-pass').value);
    btn.disabled = false;
    btn.textContent = 'ВОЙТИ';
    if (!res.ok) {
      err.textContent = res.error || 'Ошибка входа';
      err.hidden = false;
      return;
    }
    $('ely-pass').value = '';
    S.current = res.account;
    S.accounts = await api.getAccounts();
    renderAccountChip();
    renderAccountModal();
    toast(`Добро пожаловать, ${res.account.name}!`, 'ok');
  };

  $('form-offline').onsubmit = async (e) => {
    e.preventDefault();
    const res = await api.loginOffline($('off-nick').value);
    const err = $('off-error');
    if (!res.ok) {
      err.textContent = res.error || 'Ошибка';
      err.hidden = false;
      return;
    }
    err.hidden = true;
    S.current = res.account;
    S.accounts = await api.getAccounts();
    renderAccountChip();
    renderAccountModal();
    toast(`Офлайн-профиль: ${res.account.name}`, 'ok');
  };
}

// ---------------------------------------------------------------- публикация режимов

function fillPublishForm() {
  const s = S.settings || {};
  $('pub-repo').value = s.githubRepo || 'nodura1488-sudo/MODES';
  $('pub-token').value = s.githubToken || '';
  $('pub-loader').value = 'vanilla';
  $('pub-loader-version-field').hidden = true;
  $('pub-loader-version').value = '';
  $('pub-error').hidden = true;
  $('pub-progress-wrap').hidden = true;
}

function bindPublish() {
  $('btn-publish').onclick = () => openModal('modal-publish');
  $('pub-token-link').onclick = () => api.openUrl('https://github.com/settings/tokens/new?scopes=repo&description=TDEV%20Launcher');

  $('pub-loader').onchange = () => {
    $('pub-loader-version-field').hidden = $('pub-loader').value === 'vanilla';
  };

  $('pub-pick-art').onclick = async () => {
    const p = await api.chooseImage();
    if (!p) return;
    S.pub.imagePath = p;
    $('pub-art-name').textContent = p.split(/[\\/]/).pop();
    const data = await api.readImagePreview(p);
    const img = $('pub-art-preview');
    if (data) { img.src = data; img.hidden = false; }
  };

  $('pub-pick-zip').onclick = async () => {
    const p = await api.chooseZip();
    if (!p) return;
    S.pub.zipPath = p;
    const st = await api.statFile(p);
    $('pub-zip-name').textContent = `${p.split(/[\\/]/).pop()} (${st.human})`;
  };

  $('btn-publish-submit').onclick = submitPublish;
}

async function submitPublish() {
  const err = $('pub-error');
  err.hidden = true;
  const btn = $('btn-publish-submit');
  const form = {
    repo: $('pub-repo').value.trim(),
    token: $('pub-token').value.trim(),
    title: $('pub-title').value.trim(),
    version: $('pub-version').value.trim() || '1.0.0',
    mcVersion: $('pub-mc').value.trim() || '1.20.1',
    loader: $('pub-loader').value,
    loaderVersion: $('pub-loader-version').value.trim() || null,
    tags: $('pub-tags').value.split(',').map((s) => s.trim()).filter(Boolean),
    description: $('pub-desc').value.trim(),
    changelog: $('pub-changelog').value.trim(),
    accent: $('pub-accent').value
  };
  if (!form.repo || !form.token || !form.title) {
    err.textContent = 'Заполни репозиторий, токен и название режима';
    err.hidden = false;
    return;
  }
  btn.disabled = true;
  $('pub-progress-wrap').hidden = false;
  setPubProgress(0);
  $('pub-step').textContent = 'Старт…';

  const res = await api.publishMode({ form, imagePath: S.pub.imagePath, zipPath: S.pub.zipPath });

  btn.disabled = false;
  if (!res.ok) {
    $('pub-progress-wrap').hidden = true;
    err.textContent = res.error || 'Ошибка публикации';
    err.hidden = false;
    return;
  }
  S.modes = await api.getModes();
  S.state = await api.getState();
  renderModes();
  if (res.mode) selectMode(res.mode.id);
  closeModals();
  toast(`Режим «${res.mode.title}» опубликован 🎉`, 'ok');
}

function onPublishEvent(ev) {
  if (!ev) return;
  if (ev.type === 'step') {
    $('pub-progress-wrap').hidden = false;
    $('pub-step').textContent = ev.payload;
  } else if (ev.type === 'progress') {
    const p = ev.payload && ev.payload.total ? Math.round((ev.payload.task / ev.payload.total) * 100) : 0;
    $('pub-progress-wrap').hidden = false;
    $('pub-step').textContent = `Загрузка файлов… ${p}%`;
    setPubProgress(p);
  } else if (ev.type === 'error') {
    $('pub-step').textContent = 'Ошибка: ' + ev.payload;
  } else if (ev.type === 'done') {
    setPubProgress(100);
    $('pub-step').textContent = 'Готово!';
  }
}

function setPubProgress(p) {
  $('pub-progress-bar').style.width = Math.max(0, Math.min(100, p)) + '%';
}

// ---------------------------------------------------------------- утилиты

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
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

// Заглушка для открытия index.html в обычном браузере (без Electron)
function mockApi() {
  let settings = { ramGb: 4, dir: 'C:/Games/.tdevlauncher', javaPath: '', width: 925, height: 530, fullscreen: false };
  let state = {};
  const modes = [
    { id: 'demo', title: 'DEMO MODE', mcVersion: '1.20.1', version: '1.0.0', size: '—', players: '—',
      tags: ['Превью'], image: null, accent: '#7c5cff',
      description: 'Запусти лаунчер через npm start, чтобы увидеть настоящие данные.',
      changelog: 'Это превью в браузере' }
  ];
  const delay = (v) => new Promise((r) => setTimeout(() => r(v), 60));
  return {
    minimize() {}, toggleMax() {}, close() {},
    getModes: () => delay(modes),
    refreshModes: () => delay([...modes]),
    getState: () => delay(state),
    getSettings: () => delay(settings),
    saveSettings: (p) => { settings = { ...settings, ...p }; return delay(settings); },
    chooseDir: () => delay(null),
    openUrl() {}, openGameFolder() {},
    getAccounts: () => delay([]),
    getCurrentAccount: () => delay(null),
    loginEly: async () => ({ ok: false, error: 'Доступно только в приложении' }),
    loginOffline: async () => ({ ok: false, error: 'Доступно только в приложении' }),
    selectAccount: (id) => delay(null),
    removeAccount: () => delay({ accounts: [], current: null }),
    refreshSession: () => delay(null),
    install: async () => {
      setTimeout(() => { state.demo = { installed: true, version: '1.0.0' }; }, 800);
      return delay({ ok: true });
    },
    play: () => delay({ ok: true }),
    cancel() {},
    onGameEvent() {},
    chooseImage: () => delay(null),
    chooseZip: () => delay(null),
    readImagePreview: () => delay(null),
    statFile: () => delay({ size: 0, human: '—' }),
    publishMode: async () => ({ ok: false, error: 'Доступно только в приложении' }),
    onPublishEvent() {}
  };
}
