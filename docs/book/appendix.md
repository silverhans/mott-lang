# Приложение. Справочник

Краткая сводка всего синтаксиса для быстрого напоминания. Если хочешь формальные правила — смотри [`../spec.md`](../spec.md).

## Ключевые слова

| Слово | Чеч. | Роль |
|---|---|---|
| `fnc` | | объявление функции |
| `xilit` | хилит | `let` — объявление переменной |
| `yuxadalo` | юхадало | `return` |
| `yazde` | язде | встроенный вывод |
| `esha` | эша | встроенный ввод строки |
| `nagah sanna` | нагахь санна | `if` (двусловная пара) |
| `khi` | кхи | `else` |
| `cqachunna` | цкъачунна | `while` |
| `yallalc` | яллалц | `for` |
| `chu` | чу | `in` (связка в for-each) |
| `sac` | саца | `break` |
| `khida` | хьида | `continue` |
| `baram` | барам | длина массива/строки |
| `parse_terah` | | `deshnash` → `terah` |
| `parse_daqosh` | | `deshnash` → `daqosh` |
| `to_terah` | | числовая конверсия → `terah` |
| `to_daqosh` | | числовая конверсия → `daqosh` |
| `baqderg` | бакъдерг | `true` |
| `xarco` | харцо | `false` |
| `a` | | постфиксное AND |
| `ya` | я | инфиксное OR |

**Типы:**

| Слово | Тип |
|---|---|
| `terah` | int64 |
| `daqosh` | float64 |
| `bool` | bool |
| `deshnash` | string (UTF-8) |
| `[T]` | массив элементов T |

## Зарезервированные идентификаторы

Нельзя использовать как имена переменных, параметров, функций:

```
fnc xilit yuxadalo yazde esha
nagah sanna khi cqachunna yallalc chu
sac khida baram parse_terah parse_daqosh to_terah to_daqosh
baqderg xarco a ya
terah bool deshnash daqosh
```

Особо **`a`** и **`ya`** — короткие, легко случайно ввести. Используй `x`, `y`, `alpha` и т.п.

## Операторы и приоритеты

От высокого к низкому:

| Приоритет | Оператор | Описание | Ассоциативность |
|---|---|---|---|
| 1 | `-x`, `!x` | унарные | prefix |
| 2 | `[i]` | индексация | postfix |
| 3 | `(args)` | вызов функции | postfix |
| 4 | `*`, `/`, `%` | мультипликативные | left |
| 5 | `+`, `-` | аддитивные | left |
| 6 | `==`, `!=`, `<`, `<=`, `>`, `>=` | сравнения | left |
| 7 | `a` (постфикс, запятая) | AND | n-ary |
| 8 | `ya` | OR | n-ary |
| 9 | `=` | присваивание | только как stmt |
| 10 | `..` | range | только в `yallalc` |

## Синтаксис — краткая шпаргалка

### Функции

```mott
fnc name(p1: T1, p2: T2) -> R {
    // тело
    yuxadalo value
}

fnc void_name(x: T) {                  // без возврата
    // ...
}

fnc kort() {                           // точка входа
    // ...
}
```

### Переменные

```mott
xilit x = 5                            // вывод типа
xilit y: terah = 10                    // явный тип
y = y + 1                              // присваивание
```

### Условия

```mott
nagah sanna cond {
    // ...
} khi nagah sanna other {
    // ...
} khi {
    // ...
}
```

### Циклы

```mott
cqachunna cond {
    // while
}

yallalc x chu arr {
    // for-each array
}

yallalc i chu start..end {
    // for-each range (end exclusive)
}

// прерывания
sac                                    // break
khida                                  // continue
```

### Массивы

```mott
xilit nums: [terah] = [1, 2, 3]
xilit first = nums[0]                  // read
nums[0] = 42                           // write
xilit len = baram(nums)                // length
```

### Строки

```mott
xilit s: deshnash = "salam, {name}!"   // интерполяция — только идентификаторы
xilit eq = s == "hello"                // сравнение
xilit n = baram(s)                     // длина в байтах
```

### Логика

```mott
// AND — постфиксное `a`, минимум 2 конъюнкта, запятая между:
nagah sanna x > 0 a, x < 10 a {
    // ...
}

// OR — инфиксное `ya`, n-арное:
nagah sanna x == 0 ya x == 5 ya x == 10 {
    // ...
}

// NOT — унарное `!`:
nagah sanna !ready {
    // ...
}
```

### I/O

```mott
yazde(expr)                            // печать, +\n
xilit line: deshnash = esha()          // одна строка с stdin
xilit n: terah = parse_terah(line)     // строка → целое
xilit x: daqosh = parse_daqosh(line)   // строка → float
```

### Числовые конверсии

```mott
xilit x: daqosh = to_daqosh(42)        // 42 → 42.0
xilit n: terah = to_terah(3.7)         // 3.7 → 3 (к нулю)
xilit avg: daqosh = to_daqosh(sum) / to_daqosh(count)
```

Между `terah` и `daqosh` в обе стороны. На `deshnash`/`bool` не работает.

## Escape-последовательности в строках

```
\n   перевод строки
\t   таб
\r   carriage return
\"   кавычка
\\   обратный слэш
\{   литеральная {
\}   литеральная }
```

## Разделители инструкций

- **Перевод строки** — основной разделитель
- **`;`** — валидная альтернатива (полезно для многих statement'ов на одной строке)
- Внутри `(` ... `)` или `[` ... `]` переводы строк — пробел (многострочные выражения)

## Комментарии

```mott
// Только однострочные. Блочных /* */ в MVP нет.
```

## Шаблоны, которых ПОКА нет

Это запланировано в v0.3+, но в v0.2 работать не будет:

- `for` с range'ом вне `yallalc` (ranges — не значения)
- Пустой литерал массива `[]`
- Вложенные массивы `[[T]]`
- `push`/`pop`/`append` на массивах
- Конкатенация строк через `+`
- Конверсии между числовыми типами (`to_daqosh`, `to_terah`)
- Методы (`arr.len`, `s.upper`) — пока только функции
- Структуры и enums
- Замыкания
- Модули и `import`
- Generics

## Типичные ошибки компилятора

| Ошибка | Что значит |
|---|---|
| `expected variable name (got A)` | Использовал `a` или `ya` как имя — они зарезервированы |
| `type mismatch: X declared as T1 but initializer is T2` | Тип переменной и её значение не совпадают |
| `arithmetic type mismatch: T1 vs T2` | `+`, `-`, `*`, `/` между разными типами — сделай явный каст (но его нет, так что переделай) |
| `sac outside of cqachunna loop` | `sac` или `khida` вне цикла |
| `ordering comparison needs numeric operands` | `<`, `>` на строках — не поддерживается |
| `expected ; after declaration (got RBrace)` | Забыл перевод строки или `;` |
| `expected ( after nagah sanna` | Старый синтаксис — теперь скобки вокруг условия не нужны |

## Полезные команды

```sh
mott file.mott                         # компилировать + произвести бинарник
mott file.mott -o my_bin               # задать имя бинарника
mott file.mott --emit-c                # напечатать сгенерированный C
mott file.mott --keep-c                # оставить .c файл рядом с бинарником
```

## Ссылки

- **Формальная спецификация**: [`../spec.md`](../spec.md)
- **GitHub**: https://github.com/silverhans/mott-lang
- **Примеры**: в `examples/` репозитория

---

**[← назад к оглавлению](README.md)**
