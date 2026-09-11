# Сборка под Windows

Сборка выполняется на машине разработки. В закрытом контуре Node.js, npm, Rust и Cargo не нужны.

## Требования

- Windows x64.
- Node.js и npm.
- Rust toolchain с Cargo.
- Microsoft C++ Build Tools, необходимые Tauri.
- Для MSI должен быть доступен Windows VBSCRIPT optional feature.

Проверка:

```powershell
node --version
npm --version
rustc --version
cargo --version
```

## Первая сборка

```powershell
cd E:\gitflic-contour-sync
npm install
.\Build-Windows.bat
```

Повторные сборки после изменения исходников:

```powershell
.\Build-Windows.bat
```

Скрипт запускает production-сборку Tauri, собирает артефакты и считает SHA-256.

## Результат

```text
artifacts\windows\
├── GitFlic-Contour-Sync-portable.exe
├── GitFlic Contour Sync_<version>_x64-setup.exe
├── GitFlic Contour Sync_<version>_x64_en-US.msi
├── Start-GitFlic-Contour-Sync.bat
└── SHA256SUMS.txt
```

Общий архив:

```text
artifacts\GitFlic-Contour-Sync-v<version>-windows.zip
```

### Что передавать в закрытый контур

Для быстрого запуска достаточно `GitFlic-Contour-Sync-portable.exe`.

Для установки без доступа в интернет лучше использовать собранный setup/MSI. Конфигурация проекта использует offline WebView2 installer для Windows bundle.
