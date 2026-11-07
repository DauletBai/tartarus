<p align="center">
  <a href="https://media.githubusercontent.com/media/DauletBai/tartarus/main/assets/brand/tartarus-cover-4k.png">
    <img src="https://media.githubusercontent.com/media/DauletBai/tartarus/main/assets/brand/tartarus-cover-web.jpg" width="100%" alt="ТАРТАР и Я — от древних слоёв и письменности к современной сети обеспечения. Художественная обложка архива Tartarus.">
  </a>
</p>
<p align="center">
  <a href="https://dauletbai.github.io/tartarus/"><img src="assets/brand/badges/book.svg" alt="Книга: рабочая редакция"></a>
  <a href="research/sumerian_cuneiform/README.md"><img src="assets/brand/badges/research.svg" alt="Research: open archive"></a>
  <a href="https://github.com/DauletBai/tartarus/tree/codex/qazna-rust-provisioning/projects/qazna.org"><img src="assets/brand/badges/qazna.svg" alt="Qazna: Rust research prototype"></a>
</p>
<p align="center">
  <a href="https://github.com/DauletBai/tartarus/blob/codex/qazna-rust-provisioning/projects/qazna.org/Cargo.toml"><img src="assets/brand/badges/rust.svg" alt="Qazna minimum Rust version: 1.88"></a>
  <a href="https://github.com/DauletBai/tartarus/blob/codex/qazna-rust-provisioning/projects/qazna.org/docs/development.md"><img src="assets/brand/badges/postgresql.svg" alt="Qazna tested with PostgreSQL 16"></a>
  <a href="https://github.com/DauletBai/tartarus/actions/workflows/qazna.yml?query=branch%3Acodex%2Fqazna-rust-provisioning"><img src="https://github.com/DauletBai/tartarus/actions/workflows/qazna.yml/badge.svg?branch=codex%2Fqazna-rust-provisioning" alt="Qazna CI status on the development branch"></a>
</p>
<p align="center"><a href="https://dauletbai.github.io/tartarus/">Читать книгу</a> · <a href="research/sumerian_cuneiform/README.md">Исследование клинописи</a> · <a href="https://github.com/DauletBai/tartarus/tree/codex/qazna-rust-provisioning/projects/qazna.org">Проект Qazna</a> · <a href="assets/brand/README.md">Обложка 4К</a></p>

# Tartarus — «Тартар и Я»

Открытый исследовательский архив Даулета Баймурзина (Daulet Baimurza): рабочий PDF книги «Тартар и Я», материалы исследования шумерской клинописи и расчётная модель экономики Теңге. Книга ещё не издана. Текущая редакция PDF — 6 ноября 2026 года, редакция 0.89, 400 страниц; синхронизация книги — 6 ноября 2026 года.

Обновление 0.89: глава 26 и приложение дополнены расчётом без обязательного ИИ, формулами баланса и оптимизации, бюджетными сценариями, технологическим стеком и границами готовности. [Что изменено и зачем](book/README.md).

## Book and Qazna

*Tartar and I* is Daulet Baimurza’s Russian research essay. Its provisioning proposal motivates [Qazna](projects/qazna.org/README.md), the Rust/PostgreSQL research implementation in this repository. The book explains the ideas; the software makes selected mechanisms testable. Software tests do not establish the historical hypotheses or validate an entire monetary system. Chapter 26 describes implemented work, synthetic exercises and the remaining physical-evidence gap. Chapter 27 now distinguishes moral prerequisites, risks from external dependence and internal concentration of power, and work for future generations from predictions of inevitable success or failure.

[Read the Russian HTML edition](https://dauletbai.github.io/tartarus/) and use your browser’s page translation. Individual chapters and the complete text are available. Machine translations are not reviewed translations. The [HTML source](book/reader/index.html) and PDF are also included here.

The active implementation is on [`codex/qazna-rust-provisioning`](https://github.com/DauletBai/tartarus/tree/codex/qazna-rust-provisioning/projects/qazna.org). The separate Qazna repository is archived earlier work. The software and documentation licenses do not extend to the book or third-party illustrations.

## Книга

[Печатный PDF 6 × 9 дюймов](book/Tartar_i_Ya_print_6x9.pdf). Исходная рукопись, другие редакции, издательская переписка и самостоятельный проект языка программирования в этот репозиторий не включены.

## Исследование клинописи

- [Описание исследования](research/sumerian_cuneiform/README.md).
- [Метод](research/sumerian_cuneiform/RESEARCH_METHOD.txt).
- [Журнал](research/sumerian_cuneiform/JOURNAL.md).
- [Реестр источников](research/sumerian_cuneiform/SOURCE_REGISTER.csv).
- [Сводные выводы и ограничения](research/sumerian_cuneiform/synthesis/conclusions_for_book.txt).
- [Графический инвентарь](research/sumerian_cuneiform/graphic_inventory.html).

В папке `research/sumerian_cuneiform/` сохранены документы, фотографии, прорисовки, схемы, изображения, отчёты, таблицы и вспомогательные скрипты. Гипотезы, редакторские чтения и непосредственные наблюдения различаются в отчётах. Наличие материала в архиве не означает подтверждения всех высказанных гипотез.

Некоторые нулевые файлы сохранены как след неудачной загрузки; их нельзя считать полученными источниками. Произвольные внешние ссылки и ссылки на другие разделы исходной рабочей директории могут требовать оригинального проекта. Скрипты сохранены с исходными путями; отдельные проверки рассчитаны на наличие полной рабочей директории, включая рукопись книги.

## Экономическая модель

- [Описание, формулы и сценарии](research/МОДЕЛЬ_ЭКОНОМИКИ_ТЕҢГЕ.md).
- [Результаты вычислений JSON](research/tenge_calculations.json).
- [Скрипт расчёта](research/scripts/tenge_circulation_model.py).
- [Реестр источников книги](research/source_ledger.csv).

Для расширенного расчёта нужны Python 3, NumPy и SciPy: `python3 research/scripts/tenge_circulation_model.py --extended`. Команда обновляет файл результатов JSON. Числа описывают условное государство и сценарии, а не установленный прогноз.

## Qazna

В [projects/qazna.org](projects/qazna.org/README.md) развивается Rust-реализация модели обеспечения Теңге: назначения, циркуляция учётной меры, ресурсные резервы и подтверждение получения. Архитектура опирается на потребности и обратную связь по аналогии с живым организмом. Это исследовательский MVP с синтетическими данными.

Код Qazna лицензируется по AGPL-3.0-or-later, документация его папки — по CC BY-SA 4.0. Эти лицензии относятся только к проекту Qazna и не меняют права на книгу и исследовательский архив. Проверки проекта запускаются отдельным workflow GitHub Actions.

## Получение больших файлов

PDF, фотографии, растровые изображения и архивы хранятся через Git LFS. Для полной локальной копии:

```sh
git lfs install
git clone https://github.com/DauletBai/tartarus.git
cd tartarus
git lfs pull
```

Чтобы просматривать HTML-обзоры с локальными изображениями, откройте их после получения полной копии.

## Целостность и права

[MANIFEST.csv](MANIFEST.csv) содержит пути, размеры и SHA-256 файлов текущей публикации, кроме самого манифеста, внутреннего каталога Git и временных файлов. История редакций сохраняется в Git.

Авторский текст книги принадлежит автору. У внешних документов, фотографий и музейных изображений могут быть собственные правообладатели и условия использования; они указаны в реестре источников, когда известны. Общая открытая лицензия на весь архив не назначена.
