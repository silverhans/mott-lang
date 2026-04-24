# Глава 8. Массивы

Массив — это упорядоченная последовательность элементов **одного типа**. В Mott массивы объявляются типом `[T]`, где `T` — тип элемента.

## Литерал и тип

```mott
xilit nums: [terah] = [1, 2, 3, 4, 5]
xilit words: [deshnash] = ["salam", "marsha", "dog"]
xilit flags: [bool] = [baqderg, xarco, baqderg]
xilit prices: [daqosh] = [19.99, 5.50, 100.00]
```

Тип выводится, если указываешь только литерал:

```mott
xilit nums = [1, 2, 3]              // -> [terah]
xilit words = ["a", "b", "c"]       // -> [deshnash]
```

**Элементы должны быть одного типа** — компилятор проверяет:

```mott
xilit mixed = [1, 2, baqderg]       // ошибка: element 2: expected terah, got bool
```

**Пустой литерал `[]`** работает, но только **с явной аннотацией типа** — иначе компилятору неоткуда взять тип элемента:

```mott
xilit empty: [terah] = []            // ок
xilit still_empty = []               // ошибка: нужна аннотация типа
```

Обычно пустой массив создаётся чтоб потом наполнить его через `push` (см. ниже).

## Индексация

`arr[i]` — элемент по индексу `i` (с нуля):

```mott
xilit nums: [terah] = [10, 20, 30]
yazde(nums[0])           // 10
yazde(nums[1])           // 20
yazde(nums[2])           // 30
```

Индекс должен быть `terah`. Можно использовать выражения:

```mott
xilit i: terah = 1
yazde(nums[i])           // 20
yazde(nums[i + 1])       // 30
```

**Выход за границы не проверяется в MVP** — поведение undefined (как в C). Читай по валидным индексам.

## Мутация

Элементы можно менять через `arr[i] = value`:

```mott
fnc kort() {
    xilit nums: [terah] = [1, 2, 3]
    nums[0] = 100
    yazde(nums[0])       // 100
    yazde(nums[1])       // 2
}
```

## Динамический рост: `push` и `pop`

Массивы в Mott — **динамические**: можно добавлять и удалять элементы.

```mott
fnc kort() {
    xilit nums: [terah] = []
    push(nums, 1)
    push(nums, 2)
    push(nums, 3)
    yazde("{baram(nums)}")            // 3

    xilit last: terah = pop(nums)
    yazde("снял: {last}")              // снял: 3
    yazde("осталось: {baram(nums)}")   // осталось: 2
}
```

- **`push(arr, value)`** — statement, добавляет `value` в конец. `arr` обязан быть **голым именем переменной** — `push(nums[0], x)` или `push(f(), x)` не работают (компилятору нужен адрес, а составное выражение адреса не имеет).
- **`pop(arr) -> T`** — expression, возвращает последний элемент и укорачивает массив. На пустом — runtime abort. Проверяй через `baram(arr) > 0` заранее.

Рост массива амортизированно O(1): под капотом аллоцируется буфер с запасом, и `push` удваивает его при переполнении.

### Ограничение: не работает на параметрах

```mott
fnc helper(arr: [terah]) {
    push(arr, 99)                     // ОШИБКА компиляции
}
```

Причина — Mott передаёт массивы **по значению** (копия структуры), но указатель на данные общий. Если `push` в callee реаллоцирует буфер, у caller'а останется старый (невалидный) указатель. Это известная проблема Go slices без `&mut` из Rust. Пока у нас нет ссылок — `push`/`pop` разрешены только на **локальных** переменных.

Обходной путь — вернуть массив явно:

```mott
fnc append_one(arr: [terah], x: terah) -> [terah] {
    // Собираем новый массив (в этой функции он локальный — можно push)
    xilit result: [terah] = []
    yallalc n chu arr {
        push(result, n)
    }
    push(result, x)
    yuxadalo result
}
```

Не сильно элегантно, но работает.

## `baram` — длина

`baram(arr)` возвращает длину как `terah`:

```mott
xilit nums: [terah] = [10, 20, 30, 40]
yazde("{baram(nums)}")   // 4
```

Тот же `baram` работает и на строках (там возвращает число байтов).

## `yallalc ... chu ...` — цикл по массиву

Вместо ручной индексации через `cqachunna`, используй `yallalc`:

```mott
fnc kort() {
    xilit nums: [terah] = [5, 3, 8, 1, 9]
    yallalc n chu nums {
        yazde(n)
    }
}
// 5 3 8 1 9
```

`n` — новая переменная, связывается с каждым элементом по очереди. Тип выводится автоматически.

## `yallalc` с диапазоном

Если нужно просто счётчик от `a` до `b`, использует `start..end`:

```mott
yallalc i chu 0..5 {
    yazde("{i}")
}
// 0 1 2 3 4
```

**Полуоткрытый диапазон**: `0..5` это `0, 1, 2, 3, 4` — **без** 5. Такая же семантика как в Swift `0..<5` или Python `range(5)`.

Диапазон — это не тип первого класса, а специальная конструкция для `yallalc`. Вне цикла `0..5` писать нельзя.

## `sac` и `khida` работают и в `yallalc`

Прерывание и пропуск итерации работают так же как в `cqachunna`:

```mott
yallalc x chu [1, 2, 3, 4, 5] {
    nagah sanna x == 3 {
        sac                          // найти 3 и выйти
    }
    yazde(x)
}
// 1 2
```

```mott
yallalc i chu 0..10 {
    nagah sanna i % 2 == 0 {
        khida                        // пропустить чётные
    }
    yazde(i)
}
// 1 3 5 7 9
```

## Индексная итерация

Если нужен одновременно и индекс, и значение — итерируй по диапазону и индексируй:

```mott
xilit nums: [deshnash] = ["alpha", "beta", "gamma"]
yallalc i chu 0..baram(nums) {
    yazde("{i}: {nums[i]}")
}
// 0: alpha
// 1: beta
// 2: gamma
```

## Массивы и функции

Передать массив в функцию — обычный параметр:

```mott
fnc sum(arr: [terah]) -> terah {
    xilit total: terah = 0
    yallalc x chu arr {
        total = total + x
    }
    yuxadalo total
}

fnc kort() {
    xilit nums: [terah] = [1, 2, 3, 4, 5]
    yazde("{sum(nums)}")             // 15
}
```

**Массивы передаются по значению копии структуры, но буфер общий** (под капотом — указатель + длина + capacity). Мутация элементов через `arr[i] = x` внутри функции будет видна снаружи — потому что обе копии указывают на один и тот же буфер:

```mott
fnc double_all(arr: [terah]) {
    yallalc i chu 0..baram(arr) {
        arr[i] = arr[i] * 2
    }
}

fnc kort() {
    xilit nums: [terah] = [1, 2, 3]
    double_all(nums)
    yallalc n chu nums {
        yazde(n)
    }
}
// 2 4 6
```

## Возврат массива

Функция может вернуть массив, в том числе построенный динамически через `push`:

```mott
fnc make_squares(n: terah) -> [terah] {
    xilit result: [terah] = []
    yallalc i chu 1..n + 1 {
        push(result, i * i)
    }
    yuxadalo result
}

fnc kort() {
    xilit sq: [terah] = make_squares(5)
    yallalc x chu sq {
        yazde("{x}")
    }
    // 1, 4, 9, 16, 25
}
```

Это canonical-паттерн для "нужен массив неизвестного размера": объявить пустой, насыпать через `push`, вернуть.

## Что именно компилируется в C

Массивы в сгенерированном C — это структуры `mott_arr_terah`, `mott_arr_daqosh`, и т.д., каждая из которых содержит:
- `data` — указатель на буфер
- `len` — сколько сейчас занято
- `cap` — сколько элементов помещается без реаллокации

`baram(arr)` превращается в `arr.len` (не `.cap` — пользователя интересует именно занятая часть). Индексация — в `arr.data[i]`. `push` вызывает `mott_arr_terah_push(&arr, x)` — с адресом, потому что realloc может поменять `data`.

Если интересно, посмотри `--emit-c`:

```sh
mott examples/dyn-array.mott --emit-c | less
```

## Пример: сумма, минимум, максимум

```mott
fnc kort() {
    xilit nums: [terah] = [5, 2, 8, 1, 9, 3]

    xilit sum: terah = 0
    yallalc n chu nums {
        sum = sum + n
    }
    yazde("sum = {sum}")

    xilit min: terah = nums[0]
    xilit max: terah = nums[0]
    yallalc i chu 1..baram(nums) {
        nagah sanna nums[i] < min {
            min = nums[i]
        }
        nagah sanna nums[i] > max {
            max = nums[i]
        }
    }
    yazde("min = {min}, max = {max}")
}
// sum = 28
// min = 1, max = 9
```

## Попробуй сам

1. Напиши функцию `average(nums: [daqosh]) -> daqosh` — средне-арифметическое массива. Используй `baram` для деления.
2. Напиши функцию `contains(nums: [terah], target: terah) -> bool` — есть ли `target` в массиве. Подсказка: `yallalc` + `sac` при находке.
3. "Двойной массив": создай `[terah] = [3, 1, 4, 1, 5, 9, 2, 6]` и напечатай каждый элемент удвоенным. Просто цикл, без мутации.
4. "Поворот цифр": есть массив цифр `[1, 2, 3]`, напиши функцию, которая сдвигает все элементы на одну позицию вправо в той же переменной: `[1, 2, 3]` → `[3, 1, 2]`. Потребует временная переменная.
5. **`reverse(nums: [terah]) -> [terah]`** — вернуть новый массив в обратном порядке. Используй `pop` в цикле + `push` в новый массив. Ограничение pop на параметрах обойдёшь через локальную копию.
6. **`filter_positive(nums: [terah]) -> [terah]`** — вернуть только положительные элементы. Паттерн: пустой массив + `push` в цикле.
7. **Стек команд**: цикл читает строки. На `"push N"` — кладёт `N` в массив, на `"pop"` — снимает и печатает, на `"show"` — печатает все элементы, на `""` — выходит. Потребует `parse_terah` + строковые сравнения.
8. Посмотри `--emit-c` для простой программы с массивом. Найди строку с `mott_arr_terah_new(...)` — это вызов runtime'а. Сравни с тем, во что разворачивается `push(nums, x)`.

---

**[← предыдущая: Ввод и вывод](07-input-output.md) | [следующая: Собираем всё вместе →](09-example-project.md)**
