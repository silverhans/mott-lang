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

Until the extension is published to the marketplace, package it and install the `.vsix` via your editor's CLI:

```sh
# 1. Install the packager once, if you don't have it.
npm install -g @vscode/vsce

# 2. Build the .vsix from this directory.
cd editors/vscode/mott
vsce package

# 3. Install into your editor(s). Pick what you have:
code --install-extension mott-0.1.0.vsix      # VS Code
cursor --install-extension mott-0.1.0.vsix    # Cursor
```

If the `code` / `cursor` CLI isn't on your `$PATH`, use the full path from the app bundle, e.g. on macOS:
`/Applications/Cursor.app/Contents/Resources/app/bin/cursor --install-extension mott-0.1.0.vsix`.

Reload the editor window after install (Cmd+Shift+P → *Developer: Reload Window*). You can verify installation with `code --list-extensions | grep mott`.

> **Note:** dropping a symlink into `~/.vscode/extensions/` alone no longer works — recent VS Code versions rely on the registered-extensions list, not directory scanning. Use `--install-extension` above.

For one-shot dev testing without installing (useful when iterating on the grammar):
```sh
code --extensionDevelopmentPath="$(pwd)" .
```

## Building and running Mott files

If you open the [mott-lang repo](https://github.com/silverhans/mott-lang) in VS Code, `.vscode/tasks.json` is wired up:

- `Cmd+Shift+B` — compile the current `.mott` file with `mott`.
- Errors from the compiler show up in the *Problems* panel at the right line and column.
- A second task runs the compiled binary.

Standalone use outside the repo: make sure the `mott` binary is on your `$PATH` and invoke it directly.

## License

MIT — see [LICENSE](https://github.com/silverhans/mott-lang/blob/main/LICENSE).
