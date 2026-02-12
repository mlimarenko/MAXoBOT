# MAXoBOT

MAXoBOT — Rust SDK для разработки ботов в MAX Messenger.

Статус: `pre-1.0` (публичный API стабилизируется до `1.0.0`).

## Что реализовано

- Типизированный API-клиент для ключевых методов MAX Bot API.
- Надежность: retry, rate-limit, классификация ошибок, редактирование чувствительных данных в логах.
- Inbound: webhook-парсинг/проверка и long-polling с cursor/marker.
- Runtime для обработчиков: router/filter/middleware (`maxobot-dispatch`).
- Контрактные, интеграционные и совместимостные тесты.

## Быстрый старт

```bash
cargo test --workspace
cargo test --workspace --all-features
cargo build --workspace --all-features --examples
```

## Структура crates

- `maxobot` — фасадный crate.
- `maxobot-core` — API, модели и надежность.
- `maxobot-dispatch` — router/filter/middleware.
- `maxobot-webhook` — webhook verifier/parser/axum adapter.

## Короткая дорожная карта

- [x] Typed API surface для MAX Bot API.
- [x] Retry/rate-limit и error taxonomy.
- [x] Webhook + polling ingestion.
- [x] Dispatch runtime (router/filter/middleware).
- [x] Контрактные и интеграционные тесты.
- [ ] Зафиксировать публичный API и выпустить `1.0.0`.
- [ ] Подготовить публикацию crates на `crates.io`.
- [ ] Добавить все возможности API MAX.

## Документация

- `docs/README.md`
- `CONTRIBUTING.md`
- `SECURITY.md`
