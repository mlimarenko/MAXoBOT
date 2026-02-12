# Быстрый старт

## 1. Требования

- Rust (stable)
- Git
- Доступ к MAX Bot API токену для реальных проверок

## 2. Локальная настройка

```bash
git clone <repo-url>
cd MAXoBOT
cp .env.example .env
```

Заполните `.env` тестовыми значениями (без production-секретов).

## 3. Базовые проверки

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test --workspace --all-features
cargo build --workspace --all-features --examples
```

## 4. Запуск примеров

```bash
cargo run -p maxobot --example polling_echo --features rustls-tls
cargo run -p maxobot --example webhook_echo --features dispatch,webhook-axum,rustls-tls
cargo run -p maxobot --example botron_adapter_smoke --features botron-adapter
```

## 5. Что проверить после запуска

- Нет утечек токенов в логах.
- Ошибки API приходят в типизированном виде.
- Примеры компилируются и завершаются без panic.
