/* Tauri bridge: window.tm API для app.js */
(function () {
  const tauri = window.__TAURI__;
  if (!tauri) {
    console.error("Tauri API не найден");
    return;
  }

  const invoke = tauri.core.invoke;
  const listen = tauri.event.listen;

  window.tm = {
    // окно
    minimize: () => invoke("win_minimize"),
    close: () => invoke("win_close"),
    windowSplash: () => invoke("win_splash"),
    windowReady: () => invoke("win_ready"),

    // данные
    getModes: () => invoke("modes_get"),
    getCachedModes: () => invoke("modes_cached"),
    refreshModes: () => invoke("modes_refresh"),
    getState: () => invoke("state_get"),
    getSettings: () => invoke("settings_get"),
    saveSettings: (patch) => invoke("settings_save", { patch }),
    chooseDir: () => invoke("settings_choose_dir"),
    openUrl: (url) => invoke("shell_open", { url }),
    openGameFolder: () => invoke("game_open_folder"),
    getAppDataPath: () => invoke("app_data_path"),
    openAppFolder: () => invoke("app_open_folder"),
    checkUpdate: () => invoke("check_update"),
    downloadUpdate: (url) => invoke("download_update", { url }),
    installUpdate: (url) => invoke("install_update", { url }),
    onUpdateProgress: (cb) => listen("update:progress", (e) => cb(e.payload)),

    // аккаунты
    getAccounts: () => invoke("accounts_get"),
    getCurrentAccount: () => invoke("accounts_current"),
    deviceStart: () => invoke("accounts_device_start"),
    devicePoll: (deviceCode) => invoke("accounts_device_poll", { deviceCode }),
    openElyWindow: (url) => invoke("open_ely_window", { url }),
    closeElyWindow: () => invoke("close_ely_window"),
    selectAccount: (id) => invoke("accounts_select", { id }),
    removeAccount: (id) => invoke("accounts_remove", { id }),
    refreshSession: () => invoke("accounts_refresh"),

    // игра
    install: (modeId) => invoke("game_install", { modeId }),
    play: (modeId) => invoke("game_play", { modeId }),
    stopGame: () => invoke("game_cancel"),
    cancel: () => invoke("game_cancel"),
    uninstall: (modeId) => invoke("game_uninstall", { modeId }),
    setLibrary: (modeId, library) => invoke("game_set_library", { modeId, library }),
    addPlaytime: (modeId, seconds) => invoke("game_add_playtime", { modeId, seconds }),
    openModeFolder: (modeId) => invoke("game_open_mode_folder", { modeId }),
    onGameEvent: (cb) => listen("game:event", (e) => cb(e.payload))
  };
})();
