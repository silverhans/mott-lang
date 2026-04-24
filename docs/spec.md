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
| `nagah sanna` | условие `if` (двусловный токен) |
| `khi` | ветка `else` |
| `cqachunna` | цикл `while` |
| `sac` | прерывание цикла (`break`) |
| `khida` | переход к следующей итерации (`continue`) |
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
`fnc`, `xilit`, `yuxadalo`, `yazde`, `nagah`, `sanna`, `khi`, `cqachunna`, `sac`, `khida`, `baqderg`, `xarco`, `a`, `ya`, `terah`, `bool`, `deshnash`, `daqosh`.

Особо: однобуквенное `a` и `ya` — тоже зарезервированы (AND/OR). Если нужен параметр с таким именем — выбери другое, например `x`, `y`.

## Переменные

Объявление с инференсом типа или явной аннотацией:

```mott
xilit x = 5;                // тип выводится -> terah
xilit y: terah = 10;        // явный тип
xilit name: deshnash = "salam";
```

Переменные изменяемые (как в C). Повторное объявление в той же области запрещено.

Присваивание:
```mott
x = 42;
```

## Функции

```mott
fnc add(a: terah, b: terah) -> terah {
    yuxadalo a + b;
}

fnc greet(name: deshnash) {          // без возврата
    yazde("Salam, {name}!");
}
```

Точка входа:
```mott
fnc kort() {
    // программа начинается здесь
}
```

## Условия

```mott
nagah sanna (x < 5) {
    yazde("little");
} khi {
    yazde("big");
}
```

`khi`-ветка опциональна. `nagah sanna` лексер склеивает в один токен `IF`.

### else-if (`khi nagah sanna`)

После `khi` можно написать ещё одно `nagah sanna (...) {...}` — получается цепочка "иначе если". Синтаксический сахар поверх вложенного `nagah sanna` внутри `khi`-блока.

```mott
nagah sanna (x == 1) {
    yazde("one");
} khi nagah sanna (x == 2) {
    yazde("two");
} khi nagah sanna (x == 3) {
    yazde("three");
} khi {
    yazde("other");
}
```

Финальный `khi { ... }` опционален — цепочку можно оборвать после любого звена.

## Циклы

```mott
cqachunna (i < 10) {
    i = i + 1;
}
```

### Прерывание цикла

`sac` — немедленный выход из ближайшего `cqachunna` (как `break` в C).
`khida` — пропустить остаток тела и перейти к проверке условия (как `continue`).

```mott
cqachunna (baqderg) {
    nagah sanna (found) {
        sac;                    // выход из цикла
    }
    nagah sanna (i % 2 == 0) {
        i = i + 1;
        khida;                  // следующая итерация
    }
    yazde("{i}");
    i = i + 1;
}
```

Использование `sac` или `khida` вне цикла — ошибка компиляции.

## Операторы

**Арифметические** (только для `terah`, `daqosh`): `+ - * / %`

**Сравнения** (результат — `bool`): `== != < <= > >=`

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
nagah sanna (x > 0 a, x < 100 a) {
    yazde("в диапазоне");
}

// три условия:
nagah sanna (x > 0 a, x < 100 a, y != 0 a) {
    yazde("ok");
}
```

Требование: минимум 2 конъюнкта для AND-выражения. Единичный `expr a` — ошибка парсинга.

**Важно: ограничение в контекстах с `,`.** Парсер AND "жадный" — увидев `a` после выражения, он ожидает `, expr a` далее. Это создаёт конфликт с запятой-разделителем в аргументах функций:

```mott
yazde(x a, y a, z);   // ошибка: z ждёт трейлинг `a`
yazde((x a, y a), z); // корректно: скобки ограничивают область AND
```

Внутри скобок `if`/`while`-условия это не проблема — закрывающая `)` завершает AND:
```mott
nagah sanna (x a, y a) { ... }   // ок
```

## Интерполяция строк

Любой строковый литерал поддерживает `{ident}`:

```mott
xilit x = 5;
yazde("x = {x}");           // -> "x = 5\n"
```

Поддерживается только подстановка идентификатора, не произвольного выражения (в MVP). Выражения — через временные переменные.

Экранирование: `\{` → литеральная `{`. Прочие escape-последовательности: `\n`, `\t`, `\\`, `\"`.

## Встроенный вывод

`yazde(expr)` — печатает значение любого примитивного типа и добавляет `\n`.

```mott
yazde("hello");
yazde(42);
yazde(baqderg);
```

## Комментарии

`//` — до конца строки. Блочных нет в MVP.

## Грамматика (EBNF, неформально)

```
program        = { function } ;
function       = "fnc" IDENT "(" [ params ] ")" [ "->" type ] block ;
params         = param { "," param } ;
param          = IDENT ":" type ;
type           = "terah" | "bool" | "deshnash" | "daqosh" ;
block          = "{" { stmt } "}" ;
stmt           = let_stmt | assign_stmt | if_stmt | while_stmt
               | break_stmt | continue_stmt
               | return_stmt | print_stmt | expr_stmt ;
let_stmt       = "xilit" IDENT [ ":" type ] "=" expr ";" ;
assign_stmt    = IDENT "=" expr ";" ;
if_stmt        = "nagah sanna" "(" expr ")" block
               [ "khi" ( if_stmt | block ) ] ;
while_stmt     = "cqachunna" "(" expr ")" block ;
break_stmt     = "sac" ";" ;
continue_stmt  = "khida" ";" ;
return_stmt    = "yuxadalo" [ expr ] ";" ;
print_stmt     = "yazde" "(" expr ")" ";" ;
expr_stmt      = expr ";" ;
(* expr grammar handled by Pratt parser with precedences above *)
```

## Что НЕ входит в MVP

- Массивы, структуры, enums
- Generic-типы
- Замыкания, first-class functions
- Модули / импорты
- Сборка мусора (строки владеют буфером, простые правила)

## Пайплайн компиляции

```
file.mott → lexer → tokens → parser → AST → sema (v0.2) → backend → file.c → clang → бинарник
```
