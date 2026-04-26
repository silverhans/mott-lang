# Глава 10. Модули и стандартная библиотека

До сих пор все твои программы умещались в один файл. Но стоит написать что-то побольше — и захочется разделить код по темам: математика отдельно, парсинг строк отдельно, твоя бизнес-логика отдельно. Для этого в Mott есть **модули**.

## Импорт через `eca`

Ключевое слово **`eca`** (чеч. *"взять"*) подключает модуль:

```mott
eca math

fnc kort() {
    xilit r: daqosh = math.sqrt(2.0)
    yazde("{r}")          // 1.41421...
}
```

Что произошло:
1. `eca math` сказало компилятору: "найди файл `math.mott` и подключи его".
2. Все функции из `math.mott` стали доступны через **qualified access**: `math.sqrt`, `math.pi`, `math.sin` и т.д.
3. Компилятор подцепил соответствующий C-рантайм (`mott_math.c`) и слинковал его с твоим бинарником.

## Qualified access обязателен

```mott
eca math

fnc kort() {
    xilit r = math.sqrt(2.0)         // OK
    xilit r2 = sqrt(2.0)             // ОШИБКА: undefined function `sqrt`
}
```

Это сделано намеренно — видно где функция живёт. Хочешь использовать что-то из `math` — пишешь `math.`. Никаких сюрпризов "откуда тут эта `sqrt` взялась".

Бонус: ты можешь спокойно объявить **свою** функцию с тем же именем:

```mott
eca math

fnc sqrt(x: terah) -> terah {
    yuxadalo x * 2          // моя странная "квадратная" — просто удваивает
}

fnc kort() {
    yazde("{sqrt(5)}")              // 10 — моя версия
    yazde("{math.sqrt(5.0)}")       // 2.236 — настоящая
}
```

Они живут в разных пространствах имён (`sqrt` и `math.sqrt`), не конфликтуют.

## Стандартная библиотека: `math`

```mott
eca math

fnc kort() {
    yazde("π = {math.pi()}")
    yazde("e = {math.e()}")
    yazde("√2 = {math.sqrt(2.0)}")
    yazde("2^10 = {math.pow(2.0, 10.0)}")
    yazde("sin(π/2) = {math.sin(math.pi() / 2.0)}")
    yazde("|−5| = {math.abs_terah(-5)}")
    yazde("⌊3.7⌋ = {math.floor(3.7)}")
}
```

Полный список — в [справочнике](appendix.md#модули). Все math-функции работают с `daqosh` (кроме `abs_terah` для целых) и возвращают `daqosh`.

**Важно**: `math.floor`, `math.ceil`, `math.round` возвращают `daqosh`, а не `terah`. Если нужно целое — используй `to_terah`:

```mott
xilit n: terah = to_terah(math.floor(3.7))    // 3
```

Это согласуется с libm и оставляет за тобой решение когда переходить от float к int.

## Свои модули

Любой `.mott` файл, лежащий рядом с твоей точкой входа, можно импортировать:

```
my-project/
├── kort.mott           // твоя главная программа
└── geometry.mott       // модуль с геометрическими функциями
```

`geometry.mott`:

```mott
kep Point {
    x: daqosh,
    y: daqosh,
}

fnc origin() -> Point {
    yuxadalo Point { x: 0.0, y: 0.0 }
}

fnc distance(p: Point, q: Point) -> daqosh {
    eca math
    xilit dx: daqosh = p.x - q.x
    xilit dy: daqosh = p.y - q.y
    yuxadalo math.sqrt(dx * dx + dy * dy)
}
```

`kort.mott`:

```mott
eca geometry
eca math

fnc kort() {
    xilit a: geometry.Point = geometry.origin()
    xilit b: geometry.Point = geometry.Point { x: 3.0, y: 4.0 }
    yazde("|a-b| = {geometry.distance(a, b)}")
}
```

Сборка как обычно — компилятор сам найдёт `geometry.mott` рядом и подключит:

```
mott kort.mott -o myprog
```

**Ограничение**: пока резолвятся только модули в той же директории, что входной файл. Подкаталоги (`utils/string.mott`) — задача v0.5.

## Что искать где

Когда пишешь `eca foo`, компилятор ищет в порядке:

1. **`$MOTT_STDLIB/foo.mott`** — если переменная `MOTT_STDLIB` установлена. Полезно для тестирования и альтернативных установок.
2. **`<dist>/stdlib/foo.mott`** — встроенная stdlib (там лежит `math.mott`).
3. **`<твой каталог>/foo.mott`** — рядом с входным файлом.

Если не найдёт — `sema error: module 'foo' not found`.

## Циклические импорты запрещены

Mott отвергает циклы:

```mott
// a.mott
eca b
fnc f() {}

// b.mott
eca a              // ОШИБКА: import cycle detected at module `a`
fnc g() {}
```

Это естественное ограничение для модели "merge всё в одну программу". Если хочется делиться кодом между двумя модулями — выноси общий код в третий модуль.

## Под капотом

Что делает компилятор когда видит `eca math`:

1. **Loader** находит `math.mott` и парсит его. Получается `Program` с extern-объявлениями функций.
2. Каждая функция получает тег `module: Some("math")` — это становится частью её "квалифицированного имени" (`math.sqrt`).
3. Loader мерджит items из `math.mott` в основную программу пользователя.
4. **Sema** видит вызов `math.sqrt(2.0)` и ищет ключ `math.sqrt` в таблице функций — находит extern-сигнатуру.
5. **Codegen** эмитит forward declaration `double mott_math_sqrt(double x);` и вызов `mott_math_sqrt(2.0)`.
6. **Driver** видит что в программе использован module `math` и добавляет к `clang`:
   - Файл `runtime/mott_math.c` где `mott_math_sqrt(x) { return sqrt(x); }`
   - Флаг `-lm` для линковки libm

То есть `math.sqrt(2.0)` в Mott → `mott_math_sqrt(2.0)` в C → `sqrt(2.0)` из libm. Прямой путь без overhead'а.

Хочешь увидеть mangling — `mott file.mott --emit-c` и поищи `mott_math_`.

## Что НЕ работает (пока)

- **Visibility** (`pub`/`priv`) — все top-level функции и структуры доступны после импорта.
- **Renamed imports** (`eca math as m`) — пиши полное имя.
- **Selective imports** (`eca math (sqrt, pi)`) — импортируется весь модуль целиком.
- **Subdirectories** в путях — `eca utils/string` не поддерживается; модули должны быть рядом.
- **Импортировать struct из модуля как тип** работает в коде, но синтаксис `module.Type` в позиции типа пока не везде корректно обрабатывается. Если упрёшься — открывай issue.

Это всё планы на v0.5+.

## Попробуй сам

1. Напиши программу, которая по введённому радиусу с stdin печатает площадь и длину окружности (`math.pi()`, `parse_daqosh`, `esha`).
2. Используя `math.pow`, посчитай первые 10 степеней двойки и распечатай их.
3. Создай файл `geometry.mott` с функцией `hypotenuse(a: daqosh, b: daqosh) -> daqosh` (через `math.sqrt`). Импортируй и используй.
4. **Решение квадратного уравнения**: `kort` спрашивает три коэффициента `a, b, c`, выводит корни через дискриминант. Используй `math.sqrt` (или ругайся если дискриминант отрицательный).
5. **Радиус→объём шара**: $V = \frac{4}{3}\pi r^3$. Напиши.

---

**[← предыдущая: Структуры](09-structs.md) | [следующая: Собираем всё вместе →](11-example-project.md)**
