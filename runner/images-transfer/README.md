# Интеграция с `devops/images-transfer`

Эти файлы предназначены для отдельной ветки `repositories` проекта `devops/images-transfer` на Контуре 1.

В ветке `repositories` разместите:

```text
gitflic-ci.yaml
ci/repository-sync.sh
```

В CI/CD variables проекта задайте и замаскируйте:

```text
SOURCE_GIT_USERNAME
SOURCE_GIT_PASSWORD
TARGET_GIT_USERNAME
TARGET_GIT_PASSWORD
```

При необходимости маршрутизации задайте:

```text
SOURCE_HTTP_PROXY
TARGET_HTTP_PROXY
```

Для текущей схемы DEV через proxy, а PROD SK через туннель обычно достаточно заполнить `SOURCE_HTTP_PROXY`, оставив `TARGET_HTTP_PROXY` пустым.

Пользовательские API-токены из приложения в runner не передаются. Runner использует только отдельные CI/CD credentials проекта `images-transfer`.

Job получает через API только координаты выбранных проектов/веток, ожидаемые SHA, действие (`preview` или `push`) и выбранный runner tag.

`preview` выполняет fetch обеих веток, ancestry check, точный `git diff` и `git push --dry-run`. `push` повторяет проверку обоих SHA непосредственно перед записью и обновляет только выбранную ветку с `--force-with-lease=<target-ref>:<expected-sha>`. Полный `git push --mirror` не используется.
