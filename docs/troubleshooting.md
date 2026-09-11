# Диагностика

## `BranchNotFoundException`, HTTP 404

Если проект существует, но GitFlic показывает экран быстрой настройки нового репозитория, у проекта ещё нет веток. API веток может вернуть:

```text
BranchNotFoundException
404 Not Found
```

Это состояние пустого репозитория, а не ошибка авторизации.

## `401 Unauthorized`

Проверить API-токен.

## `403 Forbidden`

Токен распознан, но для запроса недостаточно прав.

## `error sending request for url`

Это transport-level ошибка до получения нормального HTTP-ответа. Проверить:

- DNS;
- маршрут/VPN/туннель;
- доступность TCP 443;
- доверие Windows/Linux к сертификату внутреннего GitFlic.

## Vite: `Port 5173 is already in use`

Только для режима разработки. На Windows:

```powershell
Get-NetTCPConnection -LocalPort 5173 -State Listen
Get-Process -Id <PID>
```

Если это старый Vite текущего проекта, завершить процесс и повторить `npm run tauri:dev`.

## Release-приложение и консоль

`npm run tauri:dev` запускается из терминала, поэтому терминал остаётся открытым. Release Windows GUI binary запускается без отдельного консольного окна.

## Pipeline `devops/images-transfer` не запускается

Проверить, что в DEV GitFlic существует ветка `repositories`, а в ней находятся `gitflic-ci.yaml` и `ci/repository-sync.sh` из каталога `runner/images-transfer/`.

Также токен Контура 1 должен иметь доступ к проекту `devops/images-transfer`.

## Runner job не стартует

Проверить tag в приложении и tag зарегистрированного GitFlic Runner. В `gitflic-ci.yaml` tag передаётся через `$RUNNER_TAG`.

## Runner не может fetch/push

Проверить masked CI/CD variables проекта `images-transfer`:

```text
SOURCE_GIT_USERNAME
SOURCE_GIT_PASSWORD
TARGET_GIT_USERNAME
TARGET_GIT_PASSWORD
```

и при необходимости `SOURCE_HTTP_PROXY` / `TARGET_HTTP_PROXY`.

## Обновление заблокировано после предпросмотра

Это штатная защита от гонки. Если после preview изменился SHA любой выбранной ветки, runner не выполняет push. Нужно нажать `Пересчитать`, просмотреть новый diff и только потом повторить обновление.
