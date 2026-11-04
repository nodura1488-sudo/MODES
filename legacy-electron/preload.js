'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('tdev', {
  // окно
  minimize: () => ipcRenderer.invoke('win:minimize'),
  toggleMax: () => ipcRenderer.invoke('win:toggleMax'),
  close: () => ipcRenderer.invoke('win:close'),
  onMaximized: (cb) => ipcRenderer.on('win:maximized', (e, v) => cb(v)),

  // данные
  getModes: () => ipcRenderer.invoke('modes:get'),
  refreshModes: () => ipcRenderer.invoke('modes:refresh'),
  getState: () => ipcRenderer.invoke('state:get'),
  getSettings: () => ipcRenderer.invoke('settings:get'),
  saveSettings: (patch) => ipcRenderer.invoke('settings:save', patch),
  chooseDir: () => ipcRenderer.invoke('settings:chooseDir'),
  openUrl: (url) => ipcRenderer.invoke('shell:open', url),

  // аккаунты
  getAccounts: () => ipcRenderer.invoke('accounts:get'),
  getCurrentAccount: () => ipcRenderer.invoke('accounts:current'),
  loginEly: (username, password) => ipcRenderer.invoke('accounts:login', { username, password }),
  loginOffline: (name) => ipcRenderer.invoke('accounts:offline', { name }),
  selectAccount: (id) => ipcRenderer.invoke('accounts:select', id),
  removeAccount: (id) => ipcRenderer.invoke('accounts:remove', id),
  refreshSession: () => ipcRenderer.invoke('accounts:refresh'),

  // игра
  install: (modeId) => ipcRenderer.invoke('game:install', modeId),
  play: (modeId) => ipcRenderer.invoke('game:play', modeId),
  cancel: () => ipcRenderer.invoke('game:cancel'),
  openGameFolder: () => ipcRenderer.invoke('game:openFolder'),
  onGameEvent: (cb) => ipcRenderer.on('game:event', (e, ev) => cb(ev)),

  // публикация режимов
  chooseImage: () => ipcRenderer.invoke('publish:chooseImage'),
  chooseZip: () => ipcRenderer.invoke('publish:chooseZip'),
  readImagePreview: (p) => ipcRenderer.invoke('publish:readPreview', p),
  statFile: (p) => ipcRenderer.invoke('publish:statFile', p),
  publishMode: (payload) => ipcRenderer.invoke('publish:mode', payload),
  onPublishEvent: (cb) => ipcRenderer.on('publish:event', (e, ev) => cb(ev))
});
