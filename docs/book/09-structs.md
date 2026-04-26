# Глава 9. Структуры

До сих пор у тебя были только встроенные типы: `terah`, `daqosh`, `bool`, `deshnash`, плюс массивы. Хочешь свои типы — например, точку с координатами `x` и `y`? Это и есть структуры.

В Mott структура объявляется ключевым словом **`kep`** (чеч. *"форма, образ"*).

## Объявление

```mott
kep Point {
    x: terah,
    y: terah,
}
```

Поля — список `имя: тип`, разделённые запятыми и/или переводами строки. Trailing comma можно. Регистр имени структуры обычно с заглавной буквы (просто соглашение, компилятор не требует).

Пустая структура тоже валидна, хотя редко осмысленна:

```mott
kep Marker {}
```

## Конструирование

```mott
xilit p: Point = Point { x: 3, y: 5 }
```

**Все поля обязательны** — компилятор отвергнет:

```mott
xilit p: Point = Point { x: 3 }
// sema error: struct literal `Point` missing field(s): y
```

**Порядок полей не важен**:

```mott
xilit q = Point { y: 5, x: 3 }     // тоже ок
```

Это удобно — не надо запоминать порядок объявления, а в коде сразу видно где какое значение.

## Доступ к полям

Через **точку**:

```mott
xilit p: Point = Point { x: 3, y: 5 }
yazde("({p.x}, {p.y})")           // (3, 5)
yazde("сумма: {p.x + p.y}")       // сумма: 8
```

Цепочки тоже работают — `p.field.field`:

```mott
kep Inner { v: terah }
kep Outer { i: Inner }

fnc kort() {
    xilit o: Outer = Outer { i: Inner { v: 42 } }
    yazde("{o.i.v}")              // 42
}
```

## Присваивание полю

```mott
p.x = 10
yazde("{p.x}")                    // 10
```

**Только одно поле в v0.3** — цепочки `o.i.v = 5` пока не работают:

```mott
o.i.v = 5
// parse error: chained field assignment isn't supported yet — assign
// to a local copy and write it back
```

Костыль:

```mott
xilit local: Inner = o.i
local.v = 5
o.i = local
```

Уберём ограничение в v0.4.

## Структуры в функциях

Передаются и возвращаются **по значению** (компилятор копирует структуру целиком):

```mott
kep Point { x: terah, y: terah }

fnc swap(p: Point) -> Point {
    yuxadalo Point { x: p.y, y: p.x }
}

fnc kort() {
    xilit a: Point = Point { x: 1, y: 2 }
    xilit b: Point = swap(a)
    yazde("a = ({a.x}, {a.y})")    // 1, 2 (не изменился)
    yazde("b = ({b.x}, {b.y})")    // 2, 1
}
```

Это значит мутации полей в функции **не видны снаружи**. Хочешь "обновить" — возвращай новую структуру:

```mott
fnc move_pt(p: Point, dx: terah, dy: terah) -> Point {
    yuxadalo Point { x: p.x + dx, y: p.y + dy }
}

fnc kort() {
    xilit p: Point = Point { x: 0, y: 0 }
    p = move_pt(p, 3, 4)            // p теперь (3, 4)
}
```

## Массивы структур

Полностью поддерживаются — `[Point]`, `push`, `pop`, `yallalc`:

```mott
fnc kort() {
    xilit pts: [Point] = []
    push(pts, Point { x: 1, y: 1 })
    push(pts, Point { x: 2, y: 4 })
    push(pts, Point { x: 3, y: 9 })

    yallalc q chu pts {
        yazde("({q.x}, {q.y})")
    }
}
```

Каждый элемент массива — это полноценная структура. Доступ к полям через `q.x` так же, как у обычной переменной.

## Вложенные структуры

Поле может быть структурой:

```mott
kep Point { x: terah, y: terah }

kep Line {
    start: Point,
    end: Point,
}

fnc length_squared(l: Line) -> terah {
    xilit dx: terah = l.end.x - l.start.x
    xilit dy: terah = l.end.y - l.start.y
    yuxadalo dx * dx + dy * dy
}

fnc kort() {
    xilit l: Line = Line {
        start: Point { x: 0, y: 0 },
        end: Point { x: 3, y: 4 },
    }
    yazde("{length_squared(l)}")    // 25
}
```

Объявлять структуры можно в любом порядке — компилятор сам найдёт связи. Можно сначала `Line`, потом `Point` — будет работать.

**Циклы по значению запрещены**:

```mott
kep Node {
    value: terah,
    next: Node,            // ОШИБКА: рекурсивная структура
}
```

Это создаёт бесконечный размер: `Node` содержит `Node`, который содержит `Node`, ... Если нужна связанная структура — вместо ссылок (которых пока нет) используй массив:

```mott
kep Node {
    value: terah,
    children: [Node],      // ок: массив — heap-индирекция
}
```

## Объявление без значения

Как и обычные переменные, структуру можно объявить без инициализатора — все поля zero-init:

```mott
xilit p: Point
yazde("p = ({p.x}, {p.y})")    // p = (0, 0)
p.x = 10
```

Полезно когда тебе нужен контейнер до того, как ты знаешь чем его наполнить.

## Что НЕ работает (пока)

- **`p == q` сравнение структур** — компилятор отвергает с сообщением "compare individual fields instead". Сделаем позже, когда определимся со семантикой.
- **Методы (`p.move(5)`)** — это отдельная фича, требует переосмысления функций.
- **Дефолтные значения полей** (`x: terah = 0`) — пока нельзя.
- **Цепочечные присваивания (`a.b.c = ...`)** — обходи через локальную копию.
- **`yazde(p)` напрямую** — у структур нет авто-string. Печатай поля по отдельности через интерполяцию.

## Подкапотом

Структуры в Mott компилируются в обычные C structs:

```c
typedef struct Point {
    int64_t x;
    int64_t y;
} Point;
```

Передача по значению — это `Point arg = src;` (полная копия). Для маленьких структур это дёшево; для больших — учитывай overhead.

`Point { x: 3, y: 5 }` лоуерится в C compound literal `((Point){.x=3, .y=5})`.

Хочешь увидеть как именно — `mott file.mott --emit-c`.

## Попробуй сам

1. Объяви `kep Rectangle { width: terah, height: terah }`. Напиши `area(r: Rectangle) -> terah` и `perimeter(r: Rectangle) -> terah`.
2. Напиши функцию `is_square(r: Rectangle) -> bool` — проверяет квадратный ли прямоугольник.
3. **`kep Person { name: deshnash, age: terah }`**. В `kort` создай массив `[Person]`, запушь нескольких людей, найди самого старшего и напечатай его имя.
4. **Геометрия**: `kep Triangle { a: Point, b: Point, c: Point }`. Напиши функцию `centroid(t: Triangle) -> Point` — координаты центра масс (среднее трёх вершин).
5. **Амбициозно**: `kep Stat { sum: terah, count: terah }`. Функция `add_value(s: Stat, x: terah) -> Stat` возвращает обновлённую статистику. Используй это в цикле для подсчёта среднего по массиву.

---

**[← предыдущая: Массивы](08-arrays.md) | [следующая: Модули и стандартная библиотека →](10-modules.md)**
