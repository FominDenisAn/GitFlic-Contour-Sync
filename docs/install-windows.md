# Установка и запуск под Windows

## Portable

Скопировать:

```text
GitFlic-Contour-Sync-portable.exe
```

и запустить двойным кликом. Установка не требуется.

## Setup EXE

Запустить:

```text
GitFlic Contour Sync_<version>_x64-setup.exe
```

и пройти мастер установки.

## MSI

Для ручной установки можно открыть `.msi` двойным кликом. Для административной установки:

```powershell
msiexec /i "GitFlic Contour Sync_<version>_x64_en-US.msi"
```

## Проверка SHA-256

```powershell
Get-FileHash .\GitFlic-Contour-Sync-portable.exe -Algorithm SHA256
```

Сверить значение с `SHA256SUMS.txt`.
