'use strict';

const crypto = require('crypto');

const BASE = 'https://authserver.ely.by/auth';

/**
 * Вход через Ely.by (Yggdrasil-совместимый API).
 * Логин и пароль передаются в теле запроса.
 */
async function elyLogin(username, password, clientToken) {
  const res = await fetch(`${BASE}/authenticate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      agent: { name: 'Minecraft', version: 1 },
      username,
      password,
      clientToken,
      requestUser: true
    })
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new Error(data.errorMessage || `Ошибка входа Ely.by (${res.status})`);
  }
  return data; // { accessToken, clientToken, availableProfiles, selectedProfile, user }
}

async function elyValidate(accessToken, clientToken) {
  try {
    const res = await fetch(`${BASE}/validate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ accessToken, clientToken })
    });
    return res.ok;
  } catch {
    return false;
  }
}

async function elyRefresh(accessToken, clientToken) {
  const res = await fetch(`${BASE}/refresh`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ accessToken, clientToken, requestUser: true })
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data.errorMessage || 'Сессия истекла, войдите заново');
  return data;
}

/** UUID как у офлайн-аккаунтов Minecraft: md5 от "OfflinePlayer:<ник>". */
function offlineUuid(name) {
  const hash = crypto.createHash('md5').update(`OfflinePlayer:${name}`, 'utf8').digest();
  hash[6] = (hash[6] & 0x0f) | 0x30; // версия 3
  hash[8] = (hash[8] & 0x3f) | 0x80; // variant
  const hex = hash.toString('hex');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

module.exports = { elyLogin, elyValidate, elyRefresh, offlineUuid };
