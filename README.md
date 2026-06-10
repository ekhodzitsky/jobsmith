# Jobsmith ⚒️

[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

> **AI-ассистент для поиска работы на hh.ru.**  
> Ищет вакансии, оценивает совместимость, пишет сопроводительные и собирает PDF — пока вы пьёте кофе.

---

## ✨ Возможности

| Фича | Описание |
|------|----------|
| 🤖 **AI-оценка совместимости** | Автоматический fit-score между вашим профилем и вакансией. Знает, куда тратить время, а куда — нет. |
| 📝 **Генерация CV и сопроводительных** | AI адаптирует резюме и письмо под конкретную вакансию, а не шлёт шаблон. |
| 📄 **PDF через Typst** | Профессиональная вёрстка документов. Не Word, не Google Docs — настоящий типографский PDF. |
| 🔍 **Умный поиск hh.ru** | Фильтры по городу, опыту, графику, зарплате. Работает с API HeadHunter напрямую. |
| 📊 **Трекинг откликов** | SQLite-база помнит все заявки, статусы и сгенерированные документы. |
| 🖥️ **TUI-интерфейс** | Удобный терминальный интерфейс на `ratatui`. Без браузера и таблиц Excel. |

---

## 🚀 Быстрый старт

```bash
# 1. Установите Typst (для генерации PDF)
#    https://typst.app

# 2. Установите и авторизуйте Kimi Code CLI — весь AI-пайплайн
#    (оценка, CV, письма) работает через `kimi --wire`.
#    Проверьте, что `kimi` запускается и вы вошли в аккаунт.

# 3. Склонируйте и соберите
git clone https://github.com/ekhodzitsky/jobsmith.git
cd jobsmith
cargo install --path .

# 4. Заполните профиль — интерактивный wizard
jobsmith setup

# 5. Найдите вакансии
jobsmith search "Rust developer" --area 1 --experience between3And6

# 6. Откликнитесь с AI-generated документами
jobsmith apply https://hh.ru/vacancy/123456

# 7. Следите за статусами
jobsmith list
jobsmith mark-applied 1   # после реальной отправки отклика
```

---

## 🏗️ Архитектура

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   hh.ru API │────▶│  Fit Score   │────▶│  AI Draft   │
│   (поиск)   │     │ (оценка)     │     │  (CV+письмо)│
└─────────────┘     └──────────────┘     └──────┬──────┘
                                                │
                       ┌─────────────┐         ▼
                       │  SQLite DB  │◀───  AI Review
                       │ (трекинг)   │      (ревизия)
                       └──────┬──────┘
                              │
                              ▼
                       ┌─────────────┐
                       │ Typst → PDF │
                       └─────────────┘
```

**Стек:** Rust · Tokio · rusqlite · ratatui · reqwest · serde · Typst · kimi-wire

---

## 🧠 Почему Jobsmith, а не ручной поиск?

| Ручной поиск | Jobsmith |
|--------------|----------|
| 30 мин на чтение одной вакансии | 10 сек на AI-оценку |
| Шаблонное CV на все вакансии | Уникальное резюме под каждую позицию |
| Excel для трекинга | Встроенная SQLite + TUI |
| Word / Google Docs | Типографский PDF через Typst |

---

## 📋 Команды

```bash
jobsmith setup              # Интерактивное создание профиля
jobsmith search <query>     # Поиск вакансий на hh.ru
jobsmith apply <url|id>     # AI-пайплайн: оценка → CV → письмо → PDF
jobsmith list               # Список откликов и статусов
jobsmith mark-applied <id>  # Отметить отклик как отправленный
jobsmith salary <company>   # Справочник зарплат (нужен salary_data.json, см. ниже)
jobsmith salary --role 96 --area 1   # Онлайн-статистика зарплат с hh.ru (без файла)
jobsmith reset              # Сброс профиля или данных
```

---

## ⚙️ Фильтры hh.ru

| Параметр | Примеры |
|----------|---------|
| `--area` | `1` — Москва, `2` — СПб |
| `--experience` | `noExperience` · `between1And3` · `between3And6` · `moreThan6` |
| `--employment` | `full` · `part` · `project` · `volunteer` · `probation` |
| `--schedule` | `fullDay` · `shift` · `flexible` · `remote` · `flyInFlyOut` |

---

## 📊 Справочник зарплат

Два режима:

**Онлайн** — статистика HH по профролям, файл не нужен:

```bash
jobsmith salary --role 96 --area 1        # 96 = разработчик, 1 = Москва
jobsmith salary --role 96 --area 1 --json
```

> ⚠️ `/salary_statistics` — партнёрский эндпоинт HH («Банк зарплат»): без
> соответствующего доступа API отвечает 404, и команда честно покажет эту
> ошибку. Локальный режим работает всегда.

**Локальный** — `jobsmith salary <company>` ищет по файлу `salary_data.json` в каталоге данных; без него команда сообщит об ошибке:

| Где лежит | Путь |
|-----------|------|
| macOS | `~/Library/Application Support/jobsmith/salary_data.json` |
| Linux | `~/.local/share/jobsmith/salary_data.json` |
| С флагом `--data-dir <dir>` | `<dir>/salary_data.json` |

Схема файла (поля `metadata`, `city`, `categories` опциональны):

```json
{
  "metadata": {
    "index_label": "Зарплатный индекс",
    "index_baseline": 100.0,
    "source": "откуда данные"
  },
  "companies": [
    {
      "company": "Яндекс",
      "city": "Москва",
      "categories": { "senior_rust": "300000–450000 ₽" }
    }
  ]
}
```

Наполняйте из любого удобного источника (внутренние бенчмарки, зарплатные опросы, выгрузки). Поиск нечувствителен к регистру, отбрасывает правовые формы («ООО», «АО», …) и понимает частичные совпадения: `jobsmith salary сбертех` найдёт «ООО СберТех».

---

## 🛠️ Для разработчиков

```bash
cargo test --all-features        # 90+ тестов
cargo clippy --all-targets --all-features  # zero warnings
cargo fmt
cargo doc --no-deps --all-features
```

Конвенции — см. [AGENTS.md](./AGENTS.md).  
Типизированные ошибки (`thiserror`), zero `unwrap` в продакшене, forward-compatible модели.

---

## 📄 Лицензия

MIT © [ekhodzitsky](https://github.com/ekhodzitsky)
