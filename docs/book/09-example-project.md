# Глава 9. Собираем всё вместе

Ты прошёл все базовые конструкции языка. В этой главе напишем один небольшой проект, который использует почти всё сразу.

## Задача: статистика чисел

Программа, которая:
1. Получает список заранее известных чисел.
2. Считает для них: сумму, среднее, минимум, максимум, количество чётных.
3. Выводит таблицу с результатами.
4. Дополнительно — находит простые числа в списке.

## Решение — шаг за шагом

### Шаг 1. Функции-помощники

```mott
fnc sum(nums: [terah]) -> terah {
    xilit total: terah = 0
    yallalc n chu nums {
        total = total + n
    }
    yuxadalo total
}

fnc average(nums: [terah]) -> daqosh {
    xilit total: terah = sum(nums)
    xilit n: terah = baram(nums)
    // terah / terah даст terah (с потерей дробной части), а нам нужен
    // float. to_daqosh поднимает целое в daqosh — после этого деление
    // идёт по плавающей точке.
    yuxadalo to_daqosh(total) / to_daqosh(n)
}
```

Встроенные `to_terah` и `to_daqosh` — конверсии между числовыми типами. `to_daqosh(42)` даёт `42.0`. `to_terah(3.7)` даёт `3` (отбрасывает дробную часть, округление к нулю). Обе — просто касты в C, без рантайма; никакие ошибки не бросают.

### Шаг 2. Минимум и максимум

```mott
fnc find_min(nums: [terah]) -> terah {
    xilit min: terah = nums[0]
    yallalc i chu 1..baram(nums) {
        nagah sanna nums[i] < min {
            min = nums[i]
        }
    }
    yuxadalo min
}

fnc find_max(nums: [terah]) -> terah {
    xilit max: terah = nums[0]
    yallalc i chu 1..baram(nums) {
        nagah sanna nums[i] > max {
            max = nums[i]
        }
    }
    yuxadalo max
}
```

**Замечание**: обе функции почти идентичные. В MVP мы не можем принять "оператор сравнения как параметр" — функции высшего порядка не поддерживаются. Дубликация здесь неизбежна.

### Шаг 3. Подсчёт чётных

```mott
fnc count_even(nums: [terah]) -> terah {
    xilit count: terah = 0
    yallalc n chu nums {
        nagah sanna n % 2 == 0 {
            count = count + 1
        }
    }
    yuxadalo count
}
```

### Шаг 4. Проверка на простоту

```mott
fnc is_prime(n: terah) -> bool {
    nagah sanna n < 2 {
        yuxadalo xarco
    }
    xilit d: terah = 2
    cqachunna d * d <= n {
        nagah sanna n % d == 0 {
            yuxadalo xarco
        }
        d = d + 1
    }
    yuxadalo baqderg
}
```

### Шаг 5. Печать простых из массива

```mott
fnc print_primes(nums: [terah]) {
    yazde("простые в списке:")
    xilit any_found: bool = xarco
    yallalc n chu nums {
        nagah sanna is_prime(n) {
            yazde("  {n}")
            any_found = baqderg
        }
    }
    nagah sanna !any_found {
        yazde("  (нет)")
    }
}
```

### Шаг 6. Главная функция — всё вместе

```mott
fnc kort() {
    xilit nums: [terah] = [7, 3, 11, 4, 8, 15, 2, 9, 13, 6]

    yazde("=== статистика массива ===")
    yazde("элементы:")
    yallalc n chu nums {
        yazde("  {n}")
    }

    xilit total: terah = sum(nums)
    xilit avg: daqosh = average(nums)
    xilit lo: terah = find_min(nums)
    xilit hi: terah = find_max(nums)
    xilit ev: terah = count_even(nums)
    xilit n_total: terah = baram(nums)

    yazde("")
    yazde("количество: {n_total}")
    yazde("сумма: {total}")
    yazde("среднее: {avg}")
    yazde("минимум: {lo}")
    yazde("максимум: {hi}")
    yazde("чётных: {ev}")

    yazde("")
    print_primes(nums)
}
```

## Полная программа

Собери всё в один файл `stats.mott`:

```mott
fnc sum(nums: [terah]) -> terah {
    xilit total: terah = 0
    yallalc n chu nums {
        total = total + n
    }
    yuxadalo total
}

fnc average(nums: [terah]) -> daqosh {
    yuxadalo to_daqosh(sum(nums)) / to_daqosh(baram(nums))
}

fnc find_min(nums: [terah]) -> terah {
    xilit min: terah = nums[0]
    yallalc i chu 1..baram(nums) {
        nagah sanna nums[i] < min {
            min = nums[i]
        }
    }
    yuxadalo min
}

fnc find_max(nums: [terah]) -> terah {
    xilit max: terah = nums[0]
    yallalc i chu 1..baram(nums) {
        nagah sanna nums[i] > max {
            max = nums[i]
        }
    }
    yuxadalo max
}

fnc count_even(nums: [terah]) -> terah {
    xilit count: terah = 0
    yallalc n chu nums {
        nagah sanna n % 2 == 0 {
            count = count + 1
        }
    }
    yuxadalo count
}

fnc is_prime(n: terah) -> bool {
    nagah sanna n < 2 {
        yuxadalo xarco
    }
    xilit d: terah = 2
    cqachunna d * d <= n {
        nagah sanna n % d == 0 {
            yuxadalo xarco
        }
        d = d + 1
    }
    yuxadalo baqderg
}

fnc print_primes(nums: [terah]) {
    yazde("простые в списке:")
    xilit any_found: bool = xarco
    yallalc n chu nums {
        nagah sanna is_prime(n) {
            yazde("  {n}")
            any_found = baqderg
        }
    }
    nagah sanna !any_found {
        yazde("  (нет)")
    }
}

fnc kort() {
    xilit nums: [terah] = [7, 3, 11, 4, 8, 15, 2, 9, 13, 6]

    yazde("=== статистика массива ===")
    yazde("элементы:")
    yallalc n chu nums {
        yazde("  {n}")
    }

    xilit total: terah = sum(nums)
    xilit avg: daqosh = average(nums)
    xilit lo: terah = find_min(nums)
    xilit hi: terah = find_max(nums)
    xilit ev: terah = count_even(nums)
    xilit n_total: terah = baram(nums)

    yazde("")
    yazde("количество: {n_total}")
    yazde("сумма: {total}")
    yazde("среднее: {avg}")
    yazde("минимум: {lo}")
    yazde("максимум: {hi}")
    yazde("чётных: {ev}")

    yazde("")
    print_primes(nums)
}
```

Компилируй и запускай:

```sh
mott stats.mott -o stats && ./stats
```

Вывод:

```
=== статистика массива ===
элементы:
  7
  3
  11
  4
  8
  15
  2
  9
  13
  6

количество: 10
сумма: 78
среднее: 7.8
минимум: 2
максимум: 15
чётных: 4

простые в списке:
  7
  3
  11
  2
  13
```

## Что мы использовали

- **`fnc ... -> tип`** — функции с возвратом
- **`fnc ...`** (без `->`) — void-функции
- **`xilit`** — локальные переменные, с выводом и явным типом
- **Массивы `[terah]`** — литерал, индексация, мутация недлины
- **`baram(...)`** — длина
- **`yallalc ... chu ...`** — по массиву и по range
- **`nagah sanna` / `khi`** — условия
- **`cqachunna`** — while (в `is_prime`)
- **`yuxadalo`** — возврат, в том числе ранний
- **Интерполяция строк** — в `yazde("{x}")`
- **Логические операторы** — `!any_found`, `==`, `<`, `>`
- **Арифметика** — `+`, `/`, `%`
- **Булевы литералы** — `baqderg`, `xarco`
- **`to_daqosh`** — конверсия `terah` → `daqosh` для честного среднего

Практически весь язык за одну программу!

## Что бы улучшить, будь у нас ещё фичи

- **Обобщённый `min/max`** — если бы были функции-параметры.
- **Интерактивный ввод массива** — через `esha` + `parse_terah` уже можно читать отдельные числа, но массив неизвестной длины требует динамических массивов (`push`) — их пока нет.
- **Выход на несколько уровней** — если нужно было вылезти сразу из всех циклов.

Все эти улучшения — в roadmap языка. Но уже сейчас ты можешь написать нечто полезное.

## Попробуй сам

1. Замени массив в `kort` на свои числа. Что поменяется в выводе?
2. Добавь функцию `product(nums: [terah]) -> terah` — произведение всех чисел. Обрати внимание на `0` в массиве.
3. Напиши `count_in_range(nums: [terah], lo: terah, hi: terah) -> terah` — сколько чисел попадают в диапазон `[lo, hi]` включительно. Используй AND с `a`.
4. "Гистограмма": для каждого уникального числа в массиве напечатай его и сколько раз оно встречается. Подсказка: для каждого элемента внутренним циклом считать вхождения. O(n²) но работает.
5. **Амбициозно**: отсортируй массив методом "bubble sort" на месте (с помощью мутации). Тебе нужны две вложенные `yallalc`/`cqachunna` и `nagah sanna` для обмена соседних.

---

**[← предыдущая: Массивы](08-arrays.md) | [справочник →](appendix.md)**
