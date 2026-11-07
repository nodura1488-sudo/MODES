# Lavunio Accounts and Skin Providers

> Статус: план на будущее. Сейчас лаунчер использует Ely.by-аккаунты, а API — токены авторов. Этот документ описывает, как перейти к собственным аккаунтам Lavunio и подключить внешние системы скинов.

## 1. Цель

- Дать игроку **аккаунт Lavunio** для библиотеки, отзывов, друзей, облачных сохранений и статистики.
- Не заставлять игрока отказываться от Ely.by или Minecraft/Microsoft.
- Подключить несколько **провайдеров скинов и сессий** через единый интерфейс.
- Не хранить пароли внешних сервисов.
- Оставить возможность играть офлайн, если аккаунт уже привязан.

## 2. Модель данных

```text
users
  id            uuid
  username      text unique
  email         text unique
  password_hash text
  role          player | author | moderator | admin
  created_at
  email_verified_at

identities
  id            uuid
  user_id       uuid -> users
  provider      lavunio | elyby | microsoft | ...
  external_id   text
  display_name  text
  avatar_url    text
  access_token  text nullable, encrypted
  refresh_token text nullable, encrypted
  expires_at
  created_at
  unique (provider, external_id)

sessions
  id            uuid
  user_id       uuid
  refresh_hash  text
  user_agent    text
  ip            text
  created_at
  last_used_at
  revoked_at

link_codes
  code          text
  user_id       uuid
  expires_at
  used_at

skin_profiles
  id            uuid
  identity_id   uuid
  username      text
  uuid          text
  skin_url      text
  cape_url      text
  model         classic | slim
  updated_at
```

## 3. Авторизация Lavunio

- Email + пароль.
- Пароль: **Argon2id**, уникальная соль.
- Подтверждение почты перед публикацией и социальными функциями.
- Access-токен короткий, refresh-токен долгий и хранится в виде хеша.
- Отзыв сессий и устройств.
- Позже: 2FA (TOTP), вход по passkey.
- Восстановление пароля по email.

## 4. Привязка внешних аккаунтов

Внешний аккаунт — это `identity`, а не отдельный пользователь.

- Игрок входит в Lavunio.
- Нажимает «Привязать Ely.by» или «Привязать Microsoft».
- Открывается OAuth-окно.
- После callback сохраняется `external_id`, профиль и токены.
- Один внешний аккаунт можно привязать только к одному Lavunio ID.
- Отвязка требует подтверждения и не должна ломать текущую игровую сессию.

### Ely.by

- OAuth device flow или обычный OAuth.
- Профиль и скин через Ely.by API.
- Запуск игры через authlib-injector, как сейчас.

### Microsoft / Minecraft official

- OAuth Microsoft.
- Xbox Live + XSTS.
- Minecraft services: профиль, entitlements, скин, плащ.
- Требуется регистрация приложения в Azure.
- Токены Minecraft короткоживущие, нужен refresh.
- Это самый сложный провайдер, делать после Ely.by.

### Другие провайдеры

- Добавлять через trait/интерфейс, а не хардкодом.
- Кандидаты: собственные скины Lavunio, TLauncher-совместимые сервисы (если безопасно), другие authlib-серверы.

## 5. Абстракция скинов

```rust
pub struct SkinProfile {
    pub username: String,
    pub uuid: String,
    pub skin_url: Option<String>,
    pub cape_url: Option<String>,
    pub model: SkinModel,
}

pub trait SkinProvider {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn profile(&self, identity: &Identity) -> Result<SkinProfile, Error>;
    fn validate_session(&self, identity: &Identity) -> Result<bool, Error>;
    fn refresh(&self, identity: &mut Identity) -> Result<(), Error>;
}
```

Провайдеры:

- `LavunioSkinProvider` — собственные скины, загрузка через API.
- `ElyBySkinProvider` — Ely.by.
- `MicrosoftSkinProvider` — Minecraft official.
- Позже — дополнительные authlib-провайдеры.

## 6. API

```text
POST /v1/auth/register
POST /v1/auth/login
POST /v1/auth/refresh
POST /v1/auth/logout
POST /v1/auth/verify-email
POST /v1/auth/password/reset

GET  /v1/me
GET  /v1/me/identities
POST /v1/me/identities/{provider}/link/start
POST /v1/me/identities/{provider}/link/complete
DELETE /v1/me/identities/{provider}

GET  /v1/skins/providers
GET  /v1/skins/{provider}/profile
POST /v1/skins/lavunio/upload
```

## 7. UI лаунчера

- Окно «Аккаунт»:
  - вход/регистрация Lavunio;
  - список привязанных аккаунтов;
  - статус сессии;
  - выбор активного игрового профиля;
  - предпросмотр скина и плаща.
- При запуске игры выбирается identity, через которую авторизуется Minecraft.
- Если Lavunio ID нет, можно играть через Ely.by, как сейчас.
- Если интернета нет, использовать сохранённую сессию, пока она валидна.

## 8. Безопасность

- Argon2id для паролей.
- Хеширование refresh-токенов.
- Шифрование внешних токенов at rest.
- Rate limit на login/register/reset.
- Защита от перебора и email enumeration.
- Журнал входов и привязок.
- Не логировать токены и пароли.
- Отдельные scopes для публикатора.
- Модерация скинов и имён.

## 9. Этапы

1. **A-01.** Lavunio ID: регистрация, вход, сессии, email verification.
2. **A-02.** Профиль и привязка Ely.by.
3. **A-03.** Собственные скины Lavunio.
4. **A-04.** Microsoft/Minecraft official.
5. **A-05.** UI аккаунтов в лаунчере.
6. **A-06.** Миграция текущих Ely.by-аккаунтов.
7. **A-07.** 2FA, passkeys, управление устройствами.

## 10. Открытые вопросы

- Нужен ли отдельный Lavunio-аккаунт для игры или только для магазина/сообщества?
- Какой email-сервис использовать для подтверждения и сброса?
- Делать ли собственные скины обязательными или оставить выбор провайдера?
- Нужна ли миграция старых Ely.by-аккаунтов в Lavunio ID автоматически?
- Кто модерирует пользовательские скины и имена?
- Нужен ли Microsoft-вход на первом этапе или достаточно Ely.by?

## Связанные документы

- [Авторы и команды](TEAMS.md)
- [Встроенный публикатор](PUBLISHER.md)
