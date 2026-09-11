# Architecture v0.3.0

## Компоненты

```text
Desktop APP
Vue 3 / Tauri / Rust
      |
      +-- GitFlic REST API: Контур 1
      +-- GitFlic REST API: Контур 2
      |
      +-- POST pipeline/start
              |
              v
      devops/images-transfer
      branch: repositories
              |
              v
        выбранный runner
          /         \
         /           \
   Git fetch       Git fetch/push
   Контур 1          Контур 2
```

## Preview

1. Приложение фиксирует выбранные project/branch/SHA.
2. Через REST API запускается pipeline `images-transfer` с действием `preview`.
3. Runner fetch-ит только выбранные ветки обоих контуров.
4. SHA повторно сверяются с теми, которые были показаны пользователю.
5. Runner определяет `SYNCED`, `DEV_AHEAD`, `PROD_AHEAD` или `DIVERGED` через `git merge-base --is-ancestor`.
6. При `DEV_AHEAD` формируются `git diff`, `--name-status`, `--numstat`, список commit-ов и `git push --dry-run`.
7. Артефакт pipeline загружается приложением и отображается как GitFlic-style diff.

## Push

1. Пользователь нажимает `Обновить ветку` только после успешного preview.
2. Запускается новый pipeline с теми же ожидаемыми SHA.
3. Runner заново fetch-ит source и target непосредственно перед записью.
4. Если любой SHA изменился, операция завершается без push.
5. Повторно проверяется ancestry.
6. Выполняется push только выбранной ветки с lease на точный target SHA.
7. Runner читает target ref после push и сравнивает его с source SHA.
8. Результат возвращается приложению через pipeline artifact.

## Секреты

Desktop API-токены используются только для REST API и не передаются в CI variables запускаемого pipeline.

Runner использует отдельные CI/CD variables:

```text
SOURCE_GIT_USERNAME / SOURCE_GIT_PASSWORD
TARGET_GIT_USERNAME / TARGET_GIT_PASSWORD
SOURCE_HTTP_PROXY / TARGET_HTTP_PROXY (optional)
```

## История

История в desktop APP хранится локально и содержит только метаданные операции: время, project/branch, SHA, pipeline id, результат и статистику diff. API-токены и Git credentials туда не записываются.
