# GitFlic Contour Sync

Desktop-приложение для сравнения двух GitFlic-контуров и безопасного обновления одной выбранной ветки через существующий GitFlic Runner.

**Версия:** 0.3.0  
**Стек:** Vue 3 · Vite · TypeScript · Tauri 2 · Rust  
**Платформы:** Windows x64; Linux/Astra Linux при наличии runtime-зависимостей Tauri/WebKitGTK

## Что делает приложение

```text
Контур 1                            Контур 2
GitFlic + API token                GitFlic + API token
     │                                  │
     └────── проекты / ветки ───────────┘
                     │
                     v
             выбранная ветка
                     │
                     v
          проверка на GitFlic Runner
                     │
          ┌──────────┴──────────┐
          │ ancestry + git diff │
          │ git push --dry-run  │
          └──────────┬──────────┘
                     │
                     v
          предпросмотр изменений
                     │
                     v
        повторная проверка обоих SHA
                     │
                     v
       push только выбранной ветки
                     │
                     v
             итоговая проверка
```

## Реализовано в 0.3.0

| Возможность | Статус |
|---|---|
| Авторизация GitFlic по API-токену | Реализовано |
| Получение пользователя, проектов и веток | Реализовано |
| Обработка пустого Git-репозитория | Реализовано |
| RU / ENG | Реализовано |
| Light / System / Dark | Реализовано |
| Выбор конкретной ветки в обоих контурах | Реализовано |
| Runner-side ancestry check | Реализовано |
| Точный Git diff перед записью | Реализовано |
| `git push --dry-run` перед записью | Реализовано |
| Защита от гонки по SHA | Реализовано |
| Push только выбранной ветки | Реализовано |
| Post-push проверка SHA | Реализовано |
| Отображение pipeline/jobs | Реализовано |
| Локальная история операций без токенов | Реализовано |

## Что нужно один раз настроить в `devops/images-transfer`

Приложение запускает pipeline в проекте `devops/images-transfer`, ветка `repositories`.

Готовые файлы находятся здесь:

```text
runner/images-transfer/
├── gitflic-ci.yaml
└── ci/
    └── repository-sync.sh
```

Их нужно разместить в отдельной ветке `repositories` проекта `devops/images-transfer`.

В CI/CD variables проекта должны быть отдельные masked credentials для Git-доступа runner:

```text
SOURCE_GIT_USERNAME
SOURCE_GIT_PASSWORD
TARGET_GIT_USERNAME
TARGET_GIT_PASSWORD
```

При необходимости отдельно задаются:

```text
SOURCE_HTTP_PROXY
TARGET_HTTP_PROXY
```

Пользовательские API-токены из desktop-приложения в runner не передаются.

## Сборка

Windows:

```powershell
npm install
.\Build-Windows.bat
```

Linux/Astra Linux:

```bash
npm install
chmod +x ./Build-Linux.sh
./Build-Linux.sh
```

## Документация

- [docs/README.md](docs/README.md) - навигация по документации
- [docs/usage.md](docs/usage.md) - работа с приложением
- [docs/runner-integration.md](docs/runner-integration.md) - настройка `images-transfer`
- [docs/build-windows.md](docs/build-windows.md) - сборка Windows
- [docs/build-linux-astra.md](docs/build-linux-astra.md) - сборка Linux/Astra
- [docs/install-windows.md](docs/install-windows.md) - установка Windows
- [docs/install-astra-linux.md](docs/install-astra-linux.md) - установка Astra Linux
- [docs/api-tokens.md](docs/api-tokens.md) - API-токены
- [docs/troubleshooting.md](docs/troubleshooting.md) - диагностика
- [ARCHITECTURE.md](ARCHITECTURE.md) - архитектура

## Безопасность

- API-токены не сохраняются приложением в постоянное хранилище.
- Git credentials runner хранятся только как masked CI/CD variables проекта `images-transfer`.
- Приложение не использует `git push --mirror`.
- Push разрешается только когда Контур 2 является предком выбранного состояния Контура 1.
- Перед реальным push runner повторно получает обе ветки и сравнивает их с SHA, которые пользователь уже видел.
- Для target ref используется `--force-with-lease=<ref>:<expected-sha>`; если target изменился, запись отклоняется.
- После push runner повторно читает target ref и проверяет, что он равен source SHA.
