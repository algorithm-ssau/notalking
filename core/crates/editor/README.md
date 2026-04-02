# Editor (`crates/editor`)

Библиотека доменной модели заметки: блоки со связями «предыдущий / следующий», обёртка контента и текстовый редактор на чанках со стилями. Крейт **без сети и БД** — его можно собирать и тестировать отдельно от Core Service.

Сейчас в `Content` реализован только текстовый вариант; расширяемые типы блоков (списки, медиа) и лимиты вложенности описаны в целевой архитектуре — **[AGENTS.md](../../../AGENTS.md)** (раздел Editor Module).

| Модуль | Назначение |
|--------|------------|
| `block` | `Block<M>` — идентификатор, `prev_id` / `next_id`, контент, метаданные `M`, метки времени |
| `content` | `Content` — варианты содержимого блока (сейчас только `Text`) |
| `text` | `TextBlock` — чанки текста, вставка/удаление, форматирование, запрос статуса по диапазону |

## Структура crate

```
editor/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs      # pub mod block, content, text
    ├── block.rs    # Block<M>
    ├── content.rs  # Content
    └── text.rs     # TextBlock, Chunk, Style, форматирование
```

## Модель данных (кратко)

- **Блок** (`Block<M>`): UUID, двусвязный список через `prev_id` и `next_id`, поле `metadata` задаётся типом `M` (по умолчанию `()`).
- **Контент** (`Content::Text(TextBlock)`): единственный вариант на данный момент.
- **Текст** (`TextBlock`): последовательность `Chunk { text, style }`. Соседние чанки с одинаковым `Style` сливаются после операций.
- **Стиль** (`Style`): опциональные `bold`, `italic`, `color`; `merge` накладывает поля «поверх» существующих.

Публичные операции над текстом: `insert_text`, `delete_range`, `delete_at` (назад/вперёд), `enable_formatting` / `disable_formatting`, `get_formatting` (ключи `bold`, `italic`, `color` и статус «включено / смешанное»).

## Разработка

Из каталога `core` репозитория:

```bash
cd core
cargo build -p editor
cargo test -p editor
```

Подключение в другом крейте workspace (как в Core Service):

```toml
editor = { path = "crates/editor" }
```
