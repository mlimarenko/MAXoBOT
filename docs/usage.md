# Базовое использование

## Основные crates

- `maxobot` — фасадный crate для большинства пользователей.
- `maxobot-core` — typed API, модели, retry/rate-limit.
- `maxobot-dispatch` — router/filter/middleware для inbound updates.
- `maxobot-webhook` — проверка и парсинг webhook запросов.
- `maxobot-botron-adapter` — интеграционный слой для Botron.

## Минимальный outbound сценарий

1. Создайте `ClientConfig` и `BotCredentials`.
2. Создайте клиента через `new_reqwest_bot_client`.
3. Вызовите typed метод (например, `send_message`).
4. Обработайте `ApiError` без скрытых fallback.

## Минимальный inbound сценарий

1. Настройте webhook endpoint.
2. Валидируйте секрет/подпись.
3. Парсите payload в typed envelope.
4. Передавайте update в dispatch pipeline.

## Рекомендации

- Используйте webhook как основной режим в проде.
- Включайте polling только для разработки и controlled fallback.
- Всегда храните токены в окружении/секрет-менеджере.
