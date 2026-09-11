# Changelog

## 0.3.0 - 2026-09-11

- Реализован runner-side ancestry check.
- Реализован точный Git diff и предпросмотр изменений.
- Добавлен `git push --dry-run` перед записью.
- Реализована повторная проверка source/target SHA перед push.
- Реализован push только выбранной ветки с lease на ожидаемый target SHA.
- Добавлена post-push проверка target SHA.
- Добавлены вкладки выполнения и локальной истории.
- Добавлены runner-файлы для ветки `repositories` проекта `devops/images-transfer`.
- Обработка пустого Git-репозитория больше не считается ошибкой веток.

## 0.2.4 - 2026-09-11

- Документация и release scripts для Windows/Linux.
