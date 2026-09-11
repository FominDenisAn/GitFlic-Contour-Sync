# Интеграция runner с `devops/images-transfer`

## Ветка

Создать в DEV GitFlic в проекте `devops/images-transfer` отдельную ветку:

```text
repositories
```

В неё скопировать содержимое каталога `runner/images-transfer/` из этого проекта:

```text
gitflic-ci.yaml
ci/repository-sync.sh
```

## CI/CD variables

В настройках CI/CD проекта `devops/images-transfer` создать masked variables:

```text
SOURCE_GIT_USERNAME
SOURCE_GIT_PASSWORD
TARGET_GIT_USERNAME
TARGET_GIT_PASSWORD
```

Если DEV GitFlic достигается через proxy, добавить:

```text
SOURCE_HTTP_PROXY=http://10.0.0.241:2270
```

Если Контур 2 идёт напрямую через tunnel, `TARGET_HTTP_PROXY` оставить незаданным.

## Требования к runner

Runner должен иметь:

```text
bash
git
awk
```

и сетевой маршрут к обоим GitFlic-контурам.

## Что приходит из APP

APP передаёт только project/branch, ожидаемые SHA, runner tag, действие preview/push и request id. Пользовательские API-токены в pipeline variables не передаются.

## Проверка после настройки

Сначала запускать только `Сравнить изменения`. Он должен создать pipeline в `devops/images-transfer`, вернуть diff и не менять target.

Только после успешного preview становится доступно обновление выбранной ветки.
