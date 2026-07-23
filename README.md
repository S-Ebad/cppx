# cppx

A lightweight C/C++ project helper written in Rust.

cppx is **not a replacement for CMake**. It's designed to simplify common CMake workflows while keeping `CMakeLists.txt` as the source of truth.

> CMake owns the project. cppx owns the workflow.

Projects created with cppx remain normal CMake projects and can be built without cppx.

## Current Features

- Generate C/C++ projects: executable, static library, or header-only
- Optional `.clangd` config generation
- Build projects through CMake, with debug/release output directories
- Run the built executable, optionally passing extra args through
- Clean build artifacts
- Automatic project discovery (walks upward to find `CMakeLists.txt`)
- Persistent per-project settings via `.cppx.json`, merged with CLI flags (`CLI > config > default`)
- Choose a CMake generator (Ninja, Unix Makefiles, Visual Studio)
- Parallel builds, with automatic core detection
- Pass CMake cache variables (`-D`) and raw passthrough args to both the configure and build steps

## Usage

### Create a project

```bash
cppx new <PROJECT>
```

`PROJECT` is a positional path. The directory is created at that location, and the project name is taken from its final path component unless `--name` is given.
If you only pass `--std`, the language is inferred from it. If you pass both `--lang` and `--std`, they must be compatible (e.g. `--lang c --std c++20` is rejected).

Examples:

```bash
cppx new hello
cppx new hello --lang cpp --std c++23
cppx new mylib --type library --lang c
cppx new mylib --type header-only
```

- `exe` projects get a `src/` directory with a generated main source file, and an executable-oriented `CMakeLists.txt`. This is the runnable target.
- `library` projects generate stub code with no `main` symbol, so the library can be linked into an executable without colliding. There's nothing to run here.
- `header-only` projects put the library code in `include/main.hpp` (or `main.h` for C). A scratch source file is also generated at the project root purely to sanity-test the header, but it's gitignored and never committed. `CMakeLists.txt` only builds a runnable test executable from it if that file happens to be present, so cloning the repo elsewhere won't break the build even without the scratch file.

Project/executable names are sanitized separately for use as C/C++ identifiers (namespaces, function prefixes, header guards). The raw name is still used for the CMake project name and directory.

Either way, `git init` is run automatically in the new directory, and a `.cppx.json` is written with the resolved settings, including which target (if any) is runnable.

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

Build output goes under a configurable build directory, split into a debug/release subdirectory. On first build, cppx runs `cmake -B` to configure; subsequent builds skip configuration if a cache already exists. A `compile_commands.json` symlink is created at the project root for editor tooling (not supported on Windows).

`--run` builds and then runs the project's configured target. Library projects, and header-only projects with no scratch test file present, have nothing to run — cppx will tell you so rather than guessing.

Examples:

```bash
cppx build --parallel
cppx build --profile release --run
cppx build --run -- --some-flag
cppx build -D CMAKE_EXPORT_COMPILE_COMMANDS=ON
```

---

### Clean build artifacts

```bash
cppx clean
```

```bash
cppx clean --project path/to/project
```

Removes the build directory. Fails if there isn't one.

---

### Project configuration (`.cppx.json`)

Every project created with `cppx new` gets a `.cppx.json` file. `cppx build` and `cppx clean` read it automatically; CLI flags always take priority over it, and hardcoded defaults are used for anything neither specifies.

Among other settings, it stores which CMake target (if any) is the runnable one for `--run` to build and execute — set automatically at project creation, but safe to edit by hand if you rename or restructure targets in `CMakeLists.txt` yourself, since cppx never tries to guess this from the CMake files.

There's currently no CLI for editing `.cppx.json`. It's a plain JSON file, edit it directly. To remove config entirely, delete the file.

## Design Goals

cppx generates the initial project structure, but after that the generated `CMakeLists.txt` remains fully user-controlled. cppx doesn't try to hide CMake from the developer, it just removes the repetitive setup.

## Planned Features

- `cppx run`, splitting `build --run` into its own subcommand (build-then-run, cargo-style)
- `cppx config` (`--show`, `--set`, `--get`, `--unset`, `reset`) not yet decided whether this is worth it over editing `.cppx.json` directly
- Configurable Visual Studio generator version (currently hardcoded)

## Why?

CMake is powerful, but many small projects repeat the same setup: creating folders, writing boilerplate CMake files, remembering build commands, passing the same CMake arguments repeatedly. cppx aims to make that quicker without hiding CMake from you.

## Status

Early development.
