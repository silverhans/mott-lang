# Mott — Language Specification v0.1 (MVP)

Mott (*мотт*, чеч. "язык") — строго типизированный язык программирования с чеченскими ключевыми словами в латинице. Компилируется в нативный бинарник через транспиляцию в C.

## Кодировка

Исходные файлы в UTF-8. Расширение — `.mott`.

## Ключевые слова (зарезервированные)

| Слово | Роль |
|---|---|
| `fnc` | объявление функции |
| `xilit` | объявление переменной |
| `yuxadalo` | возврат из функции |
| `yazde` | встроенный вывод (like `print`) |
| `esha` | встроенный ввод строки (like `read_line`) |
| `nagah sanna` | условие `if` (двусловный токен) |
| `khi` | ветка `else` |
| `cqachunna` | цикл `while` |
| `yallalc` | цикл `for` (by each / by range) |
| `chu` | связка "в/внутри" для `yallalc` (как английское `in`) |
| `sac` | прерывание цикла (`break`) |
| `khida` | переход к следующей итерации (`continue`) |
| `baram` | длина массива или строки (`size`/`len`) |
| `baqderg` | литерал `true` |
| `xarco` | литерал `false` |
| `a` | логическое AND (постфикс + запятая) |
| `ya` | логическое OR (инфикс) |
| `terah` | тип int64 |
| `bool` | тип bool |
| `deshnash` | тип string |
| `daqosh` | тип float64 |

`kort` — *не* ключевое слово, но конвенциональное имя главной функции (аналогично `main` в C/Rust).

## Типы

MVP-примитивы:
- `terah` — знаковое 64-битное целое
- `daqosh` — 64-битное с плавающей точкой (IEEE 754)
- `bool` — `baqderg` / `xarco`
- `deshnash` — UTF-8 строка

Составные типы:
- `[T]` — массив элементов типа `T`. Длина фиксирована на момент создания, элементы можно менять (`arr[i] = x`). Вложенные массивы в MVP не поддерживаются.

## Литералы

```mott
42              // terah
-7              // terah (унарный минус)
3.14            // daqosh
"salam"         // deshnash
"x = {x}"       // deshnash с интерполяцией
baqderg         // bool true
xarco           // bool false
```

## Идентификаторы

`[a-zA-Z_][a-zA-Z0-9_]*` — только ASCII для MVP. Не могут совпадать с ключевыми словами.

**Зарезервированные имена** (не могут использоваться как идентификаторы переменных, функций или параметров):
`fnc`, `xilit`, `yuxadalo`, `yazde`, `esha`, `nagah`, `sanna`, `khi`, `cqachunna`, `yallalc`, `chu`, `sac`, `khida`, `baram`, `baqderg`, `xarco`, `a`, `ya`, `terah`, `bool`, `deshnash`, `daqosh`.

Особо: однобуквенное `a` и `ya` — тоже зарезервированы (AND/OR). Если нужен параметр с таким именем — выбери другое, например `x`, `y`.

## Разделители инструкций

Инструкции разделяются **переводом строки**. Точка с запятой (`;`) остаётся валидной как явный разделитель и полезна для записи нескольких инструкций на одной строке, но в обычном стиле не пишется.

Внутри `(...)` переводы строки — это обычный пробел. Поэтому длинные выражения переносятся так:

```mott
xilit sum = (very_long_name +
             another_name +
             third_name)
```

Если перенос за пределами скобок нужен, ставится оператор в конце строки (перенос после бинарного оператора не рвёт выражение):

```mott
xilit x = foo +
    bar +
    baz
```

## Переменные

Объявление с инференсом типа или явной аннотацией:

```mott
xilit x = 5                // тип выводится -> terah
xilit y: terah = 10        // явный тип
xilit name: deshnash = "salam"
```

Переменные изменяемые (как в C). Повторное объявление в той же области запрещено.

Присваивание:
```mott
x = 42
```

## Функции

```mott
fnc add(x: terah, y: terah) -> terah {
    yuxadalo x + y
}

fnc announce(n: terah) {             // без возврата
    yazde("value: {n}")
}
```

Точка входа:
```mott
fnc kort() {
    // программа начинается здесь
}
```

## Условия

Скобки вокруг условия не нужны — `{` отделяет его от тела.

```mott
nagah sanna x < 5 {
    yazde("little")
} khi {
    yazde("big")
}
```

`khi`-ветка опциональна. `nagah sanna` лексер склеивает в один токен `IF`.

### else-if (`khi nagah sanna`)

После `khi` можно написать ещё одно `nagah sanna ... {...}` — получается цепочка "иначе если". Синтаксический сахар поверх вложенного `nagah sanna` внутри `khi`-блока.

```mott
nagah sanna x == 1 {
    yazde("one")
} khi nagah sanna x == 2 {
    yazde("two")
} khi nagah sanna x == 3 {
    yazde("three")
} khi {
    yazde("other")
}
```

Финальный `khi { ... }` опционален. `khi` может стоять как на той же строке, что и закрывающая `}`, так и на новой — парсер пропускает разделитель между ними.

## Циклы

```mott
cqachunna i < 10 {
    i = i + 1
}
```

### Прерывание цикла

`sac` — немедленный выход из ближайшего `cqachunna` (как `break` в C).
`khida` — пропустить остаток тела и перейти к проверке условия (как `continue`).

```mott
cqachunna baqderg {
    nagah sanna found {
        sac                     // выход из цикла
    }
    nagah sanna i % 2 == 0 {
        i = i + 1
        khida                   // следующая итерация
    }
    yazde("{i}")
    i = i + 1
}
```

Использование `sac` или `khida` вне цикла — ошибка компиляции.

## Операторы

**Арифметические** (только для `terah`, `daqosh`): `+ - * / %`

**Сравнения** (результат — `bool`): `== != < <= > >=`
- `== / !=` работают на всех примитивах включая `deshnash` (побайтовое сравнение в рантайме).
- `< <= > >=` — только на числовых типах (`terah`, `daqosh`); лексикографическое сравнение строк пока не поддерживается.

**Логические**:
- `!expr` — унарное NOT
- `expr ya expr [ya expr ...]` — OR, инфикс, n-арный
- `expr a, expr a [, expr a ...]` — AND, постфиксное `a` после каждого операнда, запятая между ними

Приоритеты (от высокого к низкому):
1. Унарный `- !`
2. `* / %`
3. `+ -`
4. `== != < <= > >=`
5. `a` (AND)
6. `ya` (OR)
7. `=` (присваивание — только в виде statement)

## Логический AND — детали

Синтаксис с постфиксным `a` отражает реальную грамматику чеченского (`слово а слово а`).

```mott
nagah sanna x > 0 a, x < 100 a {
    yazde("в диапазоне")
}

// три условия:
nagah sanna x > 0 a, x < 100 a, y != 0 a {
    yazde("ok")
}
```

Требование: минимум 2 конъюнкта для AND-выражения. Единичный `expr a` — ошибка парсинга.

**Важно: ограничение в контекстах с `,`.** Парсер AND "жадный" — увидев `a` после выражения, он ожидает `, expr a` далее. Это создаёт конфликт с запятой-разделителем в аргументах функций:

```mott
yazde(x a, y a, z)   // ошибка: z ждёт трейлинг `a`
yazde((x a, y a), z) // корректно: скобки ограничивают область AND
```

Внутри условий `if`/`while` это не проблема — открывающая `{` завершает AND:
```mott
nagah sanna x a, y a { ... }   // ок
```

## Интерполяция строк

Любой строковый литерал поддерживает `{ident}`:

```mott
xilit x = 5
yazde("x = {x}")           // -> "x = 5\n"
```

Поддерживается только подстановка идентификатора, не произвольного выражения (в MVP). Выражения — через временные переменные.

Экранирование: `\{` → литеральная `{`. Прочие escape-последовательности: `\n`, `\t`, `\\`, `\"`.

## Встроенный вывод

`yazde(expr)` — печатает значение любого примитивного типа и добавляет `\n`.

```mott
yazde("hello")
yazde(42)
yazde(baqderg)
```

## Массивы

```mott
xilit nums: [terah] = [1, 2, 3]         // литерал
xilit first: terah = nums[0]             // индексация (0-based)
nums[0] = 42                             // запись по индексу
xilit n: terah = baram(nums)             // длина: 3
```

`baram(x)` работает как на массивах, так и на строках — возвращает количество элементов (для строк — байтов UTF-8).

Пустые литералы `[]` пока не поддерживаются: парсер не может вывести тип элементов. Нужен хотя бы один элемент.

## Циклы по коллекциям

`yallalc` — цикл "для каждого". Переменная связывается с очередным элементом. Связка `chu` означает "в" (как английское `in`):

```mott
yallalc x chu nums {
    yazde(x)
}
```

Поддерживаются два источника:

1. **Массив** — итерация по элементам:
   ```mott
   yallalc word chu ["salam", "marsha"] {
       yazde(word)
   }
   ```

2. **Диапазон целых `start..end`** — полуоткрытый, `end` исключается:
   ```mott
   yallalc i chu 0..10 {               // i пробегает 0, 1, ..., 9
       yazde("{i}")
   }
   ```

`sac` и `khida` работают внутри `yallalc` так же как внутри `cqachunna`.

## Встроенный ввод

`esha()` — читает одну строку из stdin и возвращает её как `deshnash`. Финальный перевод строки срезается. На EOF/ошибке возвращает пустую строку.

```mott
yazde("How are you called?")
xilit name: deshnash = esha()
yazde("Salam, {name}!")
```

В отличие от Python-стиля `input("prompt")`, промпт пишется **отдельно** через `yazde` — как в C/Rust. `esha` сама ничего не печатает.

## Комментарии

`//` — до конца строки. Блочных нет в MVP.

## Грамматика (EBNF, неформально)

`TERM` — терминатор инструкции: перевод строки (на нулевой глубине скобок) или литеральная `;`. Лексер синтезирует `;` на переводах строки после токенов, которые могут завершать выражение (literal, ident, `)`, `}`, `sac`, `khida`, `yuxadalo`), в остальных случаях перевод строки — пробел.

```
program        = { function } ;
function       = "fnc" IDENT "(" [ params ] ")" [ "->" type ] block ;
params         = param { "," param } ;
param          = IDENT ":" type ;
type           = "terah" | "bool" | "deshnash" | "daqosh"
               | "[" type "]" ;
block          = "{" { stmt } "}" ;
stmt           = let_stmt | assign_stmt | index_assign_stmt
               | if_stmt | while_stmt | for_each_stmt
               | break_stmt | continue_stmt
               | return_stmt | print_stmt | expr_stmt ;
let_stmt       = "xilit" IDENT [ ":" type ] "=" expr TERM ;
assign_stmt    = IDENT "=" expr TERM ;
if_stmt        = "nagah sanna" expr block
               [ "khi" ( if_stmt | block ) ] ;
while_stmt     = "cqachunna" expr block ;
for_each_stmt  = "yallalc" IDENT "chu" ( expr ".." expr | expr ) block ;
index_assign_stmt = IDENT "[" expr "]" "=" expr TERM ;
break_stmt     = "sac" TERM ;
continue_stmt  = "khida" TERM ;
return_stmt    = "yuxadalo" [ expr ] TERM ;
print_stmt     = "yazde" "(" expr ")" TERM ;
(* esha is a primary expression: "esha" "(" ")"  — returns deshnash *)
(* baram is a primary expression: "baram" "(" expr ")"  — returns terah *)
(* array_lit: "[" expr { "," expr } [ "," ] "]" *)
(* index: postfix "[" expr "]" on a primary *)
expr_stmt      = expr TERM ;
(* expr grammar handled by Pratt parser with precedences above *)
```

## Что НЕ входит в MVP

- Структуры, enums
- Вложенные массивы (`[[terah]]`)
- Пустые массивы (`[]` без указания типа)
- Generic-типы
- Замыкания, first-class functions
- Модули / импорты
- Сборка мусора (строки и массивы владеют буфером, в MVP утекают)

## Пайплайн компиляции

```
file.mott → lexer → tokens → parser → AST → sema (v0.2) → backend → file.c → clang → бинарник
```
