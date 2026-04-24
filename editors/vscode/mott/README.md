# Mott for VS Code

Syntax highlighting and editor support for [Mott](https://github.com/silverhans/mott-lang) — a hobby programming language with Chechen keywords.

## Features

- Syntax highlighting: keywords (`nagah sanna`, `cqachunna`, `khi`, `sac`, `khida`, `yuxadalo`, `fnc`, `xilit`), built-in `yazde`, types (`terah`, `bool`, `deshnash`, `daqosh`), booleans (`baqderg` / `xarco`), logical operators (`a` / `ya`), comments, numbers, strings with `{ident}` interpolation.
- Auto-closing brackets and quotes.
- Line-comment toggle (`Cmd+/` on macOS, `Ctrl+/` on Linux/Windows).
- Auto-indent on `{` / `}`.

## Example

```mott
fnc greet(name: deshnash) {
    nagah sanna name == "Ruslan" {
        yazde("Salam, voqsha Ruslan!")
    } khi {
        yazde("Salam, {name}!")
    }
}

fnc kort() {
    greet("Ruslan")
    greet("Madina")
}
```

## Install (local development)

Until the extension is published to the marketplace, install it from source. Pick the editor directory you use:

```sh
# VS Code
ln -sfn "$(pwd)/editors/vscode/mott" ~/.vscode/extensions/silverhans.mott-0.1.0

# Cursor
ln -sfn "$(pwd)/editors/vscode/mott" ~/.cursor/extensions/silverhans.mott-0.1.0

# Windsurf / VSCodium etc. — same idea, check your editor's extensions dir.
```

Reload the editor after symlinking (Cmd+Shift+P → *Developer: Reload Window*).

For one-shot dev testing without symlinking:
```sh
code --extensionDevelopmentPath="$(pwd)/editors/vscode/mott" .
```

## Building and running Mott files

If you open the [mott-lang repo](https://github.com/silverhans/mott-lang) in VS Code, `.vscode/tasks.json` is wired up:

- `Cmd+Shift+B` — compile the current `.mott` file with `mott`.
- Errors from the compiler show up in the *Problems* panel at the right line and column.
- A second task runs the compiled binary.

Standalone use outside the repo: make sure the `mott` binary is on your `$PATH` and invoke it directly.

## License

MIT — see [LICENSE](https://github.com/silverhans/mott-lang/blob/main/LICENSE).
