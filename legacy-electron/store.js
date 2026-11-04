'use strict';

const fs = require('fs');
const path = require('path');

/** Хранение настроек/аккаунтов/состояния режимов в JSON-файлах (userData). */
class Store {
  constructor(userDataDir, defaultGameDir) {
    this.dir = userDataDir;
    this.defaultGameDir = defaultGameDir;
    fs.mkdirSync(userDataDir, { recursive: true });

    this.settings = this._load('settings.json', {
      ramGb: 4,
      dir: defaultGameDir,
      javaPath: '',
      width: 925,        // окно игры
      height: 530,
      fullscreen: false,
      launcherWidth: 1200,  // окно лаунчера
      launcherHeight: 740,
      githubRepo: 'nodura1488-sudo/MODES'
    });
    if (!this.settings.dir) this.settings.dir = defaultGameDir;
    if (!this.settings.githubRepo) this.settings.githubRepo = 'nodura1488-sudo/MODES';
    if (!this.settings.githubRepo) this.settings.githubRepo = 'nodura1488-sudo/MODES';

    this.accounts = this._load('accounts.json', []);
    this.state = this._load('mode-state.json', {});
    this.clientToken = this._load('client-token.json', null);
    if (!this.clientToken) {
      this.clientToken = require('crypto').randomUUID();
      this._save('client-token.json', this.clientToken);
    }
    this.currentAccountId = this._load('current-account.json', null);
  }

  _load(name, fallback) {
    try {
      return JSON.parse(fs.readFileSync(path.join(this.dir, name), 'utf8'));
    } catch {
      return fallback;
    }
  }

  _save(name, data) {
    fs.writeFileSync(path.join(this.dir, name), JSON.stringify(data, null, 2), 'utf8');
  }

  saveSettings(patch) {
    this.settings = { ...this.settings, ...patch };
    this._save('settings.json', this.settings);
  }

  saveAccounts() {
    this._save('accounts.json', this.accounts);
  }

  getCurrentAccount() {
    return this.accounts.find((a) => a.id === this.currentAccountId) || null;
  }

  setCurrentAccount(id) {
    this.currentAccountId = id;
    this._save('current-account.json', id);
  }

  upsertAccount(acc) {
    const idx = this.accounts.findIndex(
      (a) => a.type === acc.type && a.name.toLowerCase() === acc.name.toLowerCase()
    );
    if (idx >= 0) {
      acc.id = this.accounts[idx].id;
      this.accounts[idx] = acc;
    } else {
      this.accounts.push(acc);
    }
    this.saveAccounts();
  }

  removeAccount(id) {
    this.accounts = this.accounts.filter((a) => a.id !== id);
    this.saveAccounts();
    if (this.currentAccountId === id) {
      this.setCurrentAccount(this.accounts.length ? this.accounts[0].id : null);
    }
  }

  setModeInstalled(modeId, version) {
    this.state[modeId] = { installed: true, version, installedAt: Date.now() };
    this._save('mode-state.json', this.state);
  }

  getClientToken() {
    return this.clientToken;
  }
}

module.exports = Store;
