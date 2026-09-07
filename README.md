# Markdown

A command and its output, as Markdown ready to paste — a plugin for
[Crook](https://github.com/theguriev/crook) that the terminal does not carry.

Every finished command in Crook is a *block*, and every block has three dots at its top right.
This plugin puts two entries behind them:

* **Copy as Markdown** — the command and what it printed, in one `console` fence.
* **Copy as a report** — the same, with a heading, the exit status, the directory and the
  branch above it. What somebody reading a bug report asks for and nobody ever pastes.

The first puts this on your clipboard:

````
```console
$ cargo test
error[E0308]: mismatched types
```
````

and the second this:

> ### `cargo test`
>
> Exited 101 in `/home/eugen/Work/crook` on `blocks-helpers`.
>
> ```console
> $ cargo test
> error[E0308]: mismatched types
> ```

Output that contains a fence of its own — `cat README.md`, a `git show` of one — is wrapped in
a longer one, because three backticks around three backticks end in the middle of somebody
else's document and what gets pasted is half a file.

The report is offered only where it would say more than a fence does: on a command whose shell
reported how it ended, or where it ran. On the far side of an `ssh` that is neither, and the
entry is not there rather than producing a heading with nothing under it.

## Install

Download `plugin.wasm` from the [latest release](https://github.com/theguriev/crook-markdown/releases/latest)
and put it where Crook looks:

```sh
mkdir -p ~/.local/share/crook/plugins/theguriev.markdown
cp plugin.wasm ~/.local/share/crook/plugins/theguriev.markdown/
```

On macOS that directory is `~/Library/Application Support/crook/plugins/`, and on Windows
`%APPDATA%\crook\plugins\`. Then start Crook, open **Settings → Plugins**, select **Markdown**,
and allow what it asks for. Nothing happens until you do — see below.

## What it is allowed to do, and what that means

A sandboxed plugin has no network, no filesystem and no thread. It has no way to reach any of
them either: it *asks*, Crook decides whether what it asked for is inside what you allowed, and
does the work on its own side of the boundary. This one asks for three things.

| It asks to | Which means |
| --- | --- |
| Read the command you run it on, and what it printed | One block. The command line and how it ended arrive with every frame of the menu you opened; what it *printed* is asked for once, when you press an entry, because output can be a megabyte. Not a session, not a pane, not history — and a request raised at any other moment is refused, because there is no menu open for it to be about. |
| See which project each tab is in | The directory the command ran in and the branch it was on, which is the sentence a report puts above the fence. Refuse it and you still get both entries; the report simply has one line fewer. |
| Read and change your clipboard | To put the Markdown there, which is the entire point. It never reads it. |

It has no network and asks for none, so nothing you copy leaves this machine — it cannot.

## Build it yourself

```sh
rustup target add wasm32-unknown-unknown
cargo test                                             # 34 tests, no wasm toolchain needed
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/markdown.wasm plugin.wasm
```

Everything except `src/sys.rs` builds for your own machine, which is how the whole plugin —
what it asks for, in what order, and what comes out — is tested by `cargo test` rather than by
installing it and watching a terminal. The imports are stubbed there and write down what was
asked for.

## How it is put together

| | |
| --- | --- |
| `crates/markdown/src/lib.rs` | The ABI: every export Crook calls, each three lines, none of which decides anything. |
| `crates/markdown/src/state.rs` | The loop: the menu says which command it is about, an entry is pressed, the output is asked for, the answer is asked to be copied. |
| `crates/markdown/src/format.rs` | What a block reads as. The part with rules in it, and the part with the tests. |
| `crates/markdown/src/view.rs` | Two labels, and which of them is worth offering for this command. No padding, no type size, no colour — Crook draws them as rows of its own menu. |
| `crates/markdown/src/sys.rs` | The imports, and the stubs that stand in for them off wasm. |
| `crates/crook_plugin_api` | A **copy** of Crook's own wire crate, vendored. `ABI_VERSION` is what keeps the two honest. |

The plugin describes two entries and never says what a menu row looks like. That is the bargain
the whole tier is built on: a plugin that described its own padding would be wrong in a theme it
was never opened in, and wrong again the day the menu changes.

It is built against **plugin API 7**, which is the version that hands a plugin the command a
menu is open on. A Crook older than that refuses it by number, at load, with a line saying which
version each side speaks.

## Licence

MIT.
