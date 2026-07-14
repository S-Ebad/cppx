# cppx

A lightweight C/C++ project helper written in Rust.

cppx is **not a replacement for CMake**. It's designed to simplify common CMake workflows while keeping `CMakeLists.txt` as the source of truth.

> CMake owns the project. cppx owns the workflow.

Projects created with cppx remain normal CMake projects and can be built without cppx.

## Current Features

- Generate C/C++ projects (executable or header-only)
- Optional `.clangd` config generation
- Build projects through CMake, with debug/release output directories
- Run the built executable, optionally passing extra args through
- Clean build artifacts
- Automatic project discovery (walks upward to find `CMakeLists.txt`)
- Pass extra build arguments straight to `cmake --build`

## Usage

### Create a project

```bash
cppx new <PROJECT>
```

`PROJECT` is a positional path — the directory is created at that location, and the project name is taken from its final path component.

Options:

| Flag | Description |
|---|---|
| `--lang <cpp\|c>` | Project language. Default: `cpp` |
| `--std <STANDARD>` | Language standard (e.g. `c++20`, `c17`). Default: latest supported for the chosen language (`c++23` / `c23`) |
| `--no-clangd` | Skip generating a `.clangd` file |
| `--header-only` | Generate a header-only library layout instead of an executable |

If you only pass `--std`, the language is inferred from it. If you pass both `--lang` and `--std`, they must be compatible (e.g. `--lang c --std c++20` is rejected).

Examples:

```bash
cppx new hello
cppx new hello --lang cpp --std c++23
cppx new mylib --header-only --lang c
```

A standard project gets a `src/` directory and an executable-oriented `CMakeLists.txt`.

A `--header-only` project puts the library code in `include/main.hpp` (or `include/main.h` for C) and generates a library-oriented CMakeLists.txt. A main.cpp/main.c is also generated at the project root as a scratch file for testing the header but it's added to .gitignore so it's never committed, and CMakeLists.txt only builds the <PROJECT>_test executable from it if that file happens to be present

Either way, `git init` is run automatically in the new directory

---

### Build a project

From inside a project:

```bash
cppx build
```

Or specify a project explicitly:

```bash
cppx build --project path/to/project
```

Options:

| Flag | Description |
|---|---|
| `-p, --project <PATH>` | Project directory. Default: walk upward from cwd to find `CMakeLists.txt` |
| `--release` | Build into `build/release` instead of `build/debug` |
| `--run` | Run the built executable after a successful build |
| `--build-args <ARGS>...` | Extra arguments passed through to `cmake --build` |
| `-- <ARGS>...` | Extra arguments passed to the executable itself (requires `--run`) |

Build output goes to `build/debug/` or `build/release/` depending on `--release`. On first build, cppx runs `cmake -B <build-dir>` to configure; subsequent builds skip configuration if a cache already exists. A `compile_commands.json` symlink is created at the project root for editor tooling.

Examples:

```bash
cppx build --build-args --parallel
cppx build --release --run
cppx build --run -- --some-flag
```

---

### Clean build artifacts

```bash
cppx clean
```

```bash
cppx clean --project path/to/project
```

Removes the `build/` directory. Fails if there isn't one.

## Design Goals

cppx generates the initial project structure, but after that the generated `CMakeLists.txt` remains fully user-controlled. cppx doesn't try to hide CMake from the developer, it just removes the repetitive setup.

## Planned Features

Design is tracked toward a `cppx.json`-based config, not yet implemented:

- `cppx config` (`--show`, `--set`, `--get`, `--unset`, `reset`)
- Persistent settings via `cppx.json`: generator, jobs, build-type, build-dir, cmake-options, compiler
- `--name` (separate executable/target name from project directory name)
- `--compiler <gcc|clang>` on `new`
- `--type <exe|library|header-only>` (replacing/extending `--header-only`)
- `build -j/--jobs`, `--profile <debug|release>` (replacing `--release`), `--no-config`, `--clean-first`, `-D <OPTION>` for CMake cache variables

## Why?

CMake is powerful, but many small projects repeat the same setup: creating folders, writing boilerplate CMake files, remembering build commands, passing the same CMake arguments repeatedly. cppx aims to make that quicker without hiding CMake from you.

## Status

Early development.
