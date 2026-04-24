# Mott for VS Code

Syntax highlighting for **Mott (мотт)** — a small, statically-typed hobby programming language with Chechen keywords that compiles to native binaries via C.

## Example

```mott
fnc square(x: terah) -> terah {
    yuxadalo x * x
}

fnc kort() {
    xilit n: terah = 7
    xilit s: terah = square(n)
    yazde("{n} squared is {s}")
}
```

And a fuller showcase — loops, `khi nagah sanna` chains, and the postfix-`a` logical AND:

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

| Mott | Meaning |
|---|---|
| `fnc` | function |
| `xilit` | variable binding (`let`) |
| `nagah sanna` | `if` |
| `khi` | `else` |
| `cqachunna` | `while` |
| `sac`, `khida` | `break`, `continue` |
| `yuxadalo` | `return` |
| `yazde` | `print` |
| `baqderg`, `xarco` | `true`, `false` |
| `a`, `ya` | logical AND (postfix), OR |
| `terah`, `daqosh`, `bool`, `deshnash` | `int64`, `float64`, `bool`, `string` |

Full language spec: [docs/spec.md](https://github.com/silverhans/mott-lang/blob/main/docs/spec.md).

## Features

- **Syntax highlighting** — keywords, types, literals, comments, and `{ident}` string interpolation.
- **Language-aware editing** — auto-closing brackets and quotes, `Cmd+/` comment toggle, auto-indent on braces.
- **File icon** — a wolf (борз, the national symbol of Chechnya) in editor tabs.

This extension is static — no LSP, autocomplete, or linting yet. Those are on the roadmap.

## Compiling Mott programs

This extension only handles highlighting. To actually build and run `.mott` files, install the `mott` compiler:

```sh
git clone https://github.com/silverhans/mott-lang
cargo install --path mott-lang/compiler
mott hello.mott && ./hello
```

Requires Rust (for the compiler) and `clang` (to link the runtime) on your machine.

If you open the [mott-lang repo](https://github.com/silverhans/mott-lang) itself in VS Code, `Cmd+Shift+B` is pre-wired to compile the active file and route errors to the *Problems* panel.

## Links

- **Repository**: https://github.com/silverhans/mott-lang
- **Language spec**: https://github.com/silverhans/mott-lang/blob/main/docs/spec.md
- **Issues / feedback**: https://github.com/silverhans/mott-lang/issues

## License

MIT — see [LICENSE](https://github.com/silverhans/mott-lang/blob/main/LICENSE).
