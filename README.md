<p align="center">
  <img src="editors/vscode/mott/icon.png" width="128" alt="Mott logo" />
</p>

<h1 align="center">Mott</h1>

<p align="center">
  A small, statically-typed programming language with Chechen keywords, compiled to native binaries via C.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License" /></a>
  <img src="https://img.shields.io/badge/version-0.2-green.svg" alt="v0.2" />
  <a href="https://marketplace.visualstudio.com/items?itemName=silverhans.mott"><img src="https://img.shields.io/badge/VS%20Code-Mott-007acc.svg" alt="VS Code Extension" /></a>
</p>

---

**Mott** (*мотт*, чеч. *язык*) — хобби-язык со словарём из чеченского: `fnc` вместо `func`, `xilit` вместо `let`, `nagah sanna` вместо `if`, `cqachunna` вместо `while`. Под капотом — транспиляция в C и сборка через `clang`, на выходе — быстрый нативный бинарник.

## Hello, world

```mott
fnc kort() {
    yazde("Salam, dunya!")
}
```

```sh
$ mott hello.mott && ./hello
Salam, dunya!
```

## Попробуй язык на вкус

FizzBuzz — показывает циклы, else-if цепочку, постфиксное чеченское "и" (AND):

```mott
fnc kort() {
    xilit i: terah = 1
    cqachunna i <= 100 {
        nagah sanna i % 3 == 0 a, i % 5 == 0 a {
            yazde("FizzBuzz")
        } khi nagah sanna i % 3 == 0 {
            yazde("Fizz")
        } khi nagah sanna i % 5 == 0 {
            yazde("Buzz")
        } khi {
            yazde("{i}")
        }
        i = i + 1
    }
}
```

Массивы и for-each — типобезопасные, с range-итерацией:

```mott
fnc kort() {
    xilit nums: [terah] = [5, 2, 8, 1, 9]
    xilit total: terah = 0
    yallalc n chu nums {
        total = total + n
    }
    yazde("sum = {total}")           // sum = 25

    yallalc i chu 0..3 {
        yazde("i = {i}")             // i = 0, i = 1, i = 2
    }
}
```

Интерактивный ввод:

```mott
fnc kort() {
    yazde("как тебя зовут?")
    xilit name: deshnash = esha()
    yazde("salam, {name}!")
}
```

## Установка

Нужны [Rust](https://rustup.rs) и `clang` (обычно уже стоит на macOS/Linux).

```sh
git clone https://github.com/silverhans/mott-lang.git
cargo install --path mott-lang/compiler
```

После этого `mott` доступен из любой директории:

```sh
mott your_file.mott           # → бинарник ./your_file
mott your_file.mott --emit-c  # посмотреть сгенерированный C
```

## Документация

- 📖 **[Учебник](docs/book/README.md)** — пошаговое введение в язык, от "Salam, dunya" до собственного проекта. 9 глав + справочник.
- 📘 **[Спецификация](docs/spec.md)** — формальная грамматика, семантика, детали лексера.
- ✍️ **[Примеры](examples/)** — рабочие `.mott` программы: fizzbuzz, primes, arrays, echo, add.

## Поддержка редактора

Расширение **Mott** для VS Code / Cursor / VSCodium:

[![Install from Marketplace](https://img.shields.io/badge/Install-Marketplace-007acc.svg)](https://marketplace.visualstudio.com/items?itemName=silverhans.mott)

Даёт подсветку синтаксиса, иконку файла (волк — национальный символ Чечни), правила авто-отступов. Для сборки из редактора открой репо — `.vscode/tasks.json` подключит `Cmd+Shift+B`.

Либо собери из исходников расширения:

```sh
cd mott-lang/editors/vscode/mott
vsce package
code --install-extension mott-0.2.0.vsix
```

## Структура репозитория

```
compiler/            # компилятор на Rust
  src/               #   lexer, parser, AST, codegen
  runtime/           #   C-рантайм (строки, массивы, I/O)
  target/            #   (cargo-артефакты, в .gitignore)
docs/
  book/              # учебник
  spec.md            # формальная спецификация
editors/vscode/mott/ # расширение для VS Code
examples/            # программы на Mott
.vscode/tasks.json   # сборка из VS Code (Cmd+Shift+B)
```

## Сборка из исходников

```sh
cd compiler
cargo build --release      # →  target/release/mott
cargo test                 # 89 тестов
cargo install --path .     # установка в ~/.cargo/bin
```

## Архитектура компилятора

```
foo.mott ─┬─> lexer ─> tokens ─> parser ─> AST ─┬─> C backend ─> foo.c ─> clang ─> foo (native)
          │                                    │
          └─────── (будущий LLVM backend) ─────┘
```

Фронтенд (лексер + парсер + AST) изолирован за trait'ом `Backend`. Сейчас есть один бэкенд — C через `clang`. Архитектура готова к добавлению LLVM, WASM или любого другого таргета без переписывания фронта.

## Статус

**v0.2** — рабочий язык с типами, функциями, управлением потоком, массивами, for-each, I/O. Смотри [CHANGELOG в git log](https://github.com/silverhans/mott-lang/commits/main) — каждый коммит описывает что добавилось.

**Следующее в roadmap** (приоритетно):
- Стандартная библиотека (`parse_terah`, `toupper`, `sqrt` и т.п.)
- Конверсии типов (`to_daqosh`, `to_terah`)
- Sema как отдельный проход (сейчас type-check живёт в кодгене)
- Динамические массивы (`push`, `pop`)
- Структуры

**Не в ближайших планах**: модули, generics, замыкания, LLVM-бэкенд — не нужны для текущих задач.

## Лицензия

[MIT](LICENSE) — делай что хочешь.
