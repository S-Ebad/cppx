//! End-to-end tests: drive the real `cppx` binary against real CMake.
//! Requires `cmake`, `ninja` and a C/C++ compiler on PATH.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn cppx(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cppx"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run cppx")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[track_caller]
fn assert_ok(out: &Output) {
    assert!(
        out.status.success(),
        "cppx failed ({})\nstdout:\n{}\nstderr:\n{}",
        out.status,
        stdout(out),
        stderr(out)
    );
}

#[test]
fn new_exe_layout() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "app"]));

    let p = tmp.path().join("app");
    for f in [
        "CMakeLists.txt",
        ".cppx.json",
        ".clangd",
        ".gitignore",
        "src/main.cpp",
    ] {
        assert!(p.join(f).is_file(), "missing {f}");
    }
    assert!(p.join("include").is_dir());

    let cmake = fs::read_to_string(p.join("CMakeLists.txt")).unwrap();
    assert!(cmake.contains("set(CMAKE_CXX_STANDARD 23)"));
}

#[test]
fn new_c_project_from_std() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(
        tmp.path(),
        &["new", "capp", "--std", "c89", "--no-clangd"],
    ));

    let p = tmp.path().join("capp");
    assert!(p.join("src/main.c").is_file());
    assert!(!p.join(".clangd").exists());

    let cmake = fs::read_to_string(p.join("CMakeLists.txt")).unwrap();
    assert!(cmake.contains("set(CMAKE_C_STANDARD 90)"));
}

#[test]
fn new_rejects_mismatched_lang_and_std() {
    let tmp = tempfile::tempdir().unwrap();
    let out = cppx(tmp.path(), &["new", "bad", "--lang", "c", "--std", "c++20"]);

    assert!(!out.status.success());
    // nothing is created when validation fails
    assert!(!tmp.path().join("bad").exists());
}

#[test]
fn new_refuses_existing_directory() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join("taken")).unwrap();

    assert!(!cppx(tmp.path(), &["new", "taken"]).status.success());
}

#[test]
fn run_exe_passes_args_and_exit_code() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "app"]));
    let p = tmp.path().join("app");

    fs::write(
        p.join("src/main.cpp"),
        "#include <cstdio>\nint main(int argc, char** argv) { std::puts(argv[argc - 1]); return argc + 2; }\n",
    )
    .unwrap();

    let out = cppx(&p, &["run", "--", "a", "last"]);
    assert_eq!(out.status.code(), Some(5), "stderr:\n{}", stderr(&out));
    assert!(stdout(&out).contains("last"));
}

#[test]
fn run_from_subdirectory_finds_project() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "app"]));

    let out = cppx(&tmp.path().join("app/src"), &["run"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("Hello from app"));
}

#[test]
fn run_uses_name_override() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "dir", "--name", "other"]));

    let out = cppx(&tmp.path().join("dir"), &["run"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("Hello from other"));
}

#[test]
fn build_writes_profile_dirs_and_compile_commands() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "app"]));
    let p = tmp.path().join("app");

    assert_ok(&cppx(&p, &["build"]));
    assert_ok(&cppx(&p, &["build", "--profile", "release", "-j", "2"]));

    assert!(p.join("build/Debug/app").is_file());
    assert!(p.join("build/Release/app").is_file());

    #[cfg(unix)]
    assert!(p.join("build/compile_commands.json").is_file());
}

#[test]
fn build_warns_about_ignored_configure_flags() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "app"]));
    let p = tmp.path().join("app");

    assert_ok(&cppx(&p, &["build"]));
    let out = cppx(&p, &["build", "-D", "FOO=1"]);
    assert_ok(&out);
    assert!(stderr(&out).contains("--reconfigure"));
}

#[test]
fn run_release_with_multi_config_generator() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "app"]));
    let p = tmp.path().join("app");

    let out = cppx(
        &p,
        &[
            "run",
            "--profile",
            "release",
            "--config-args=-GNinja Multi-Config",
        ],
    );
    assert_ok(&out);
    assert!(stdout(&out).contains("Hello from app"));

    // only the requested config is built
    assert!(p.join("build/Release/Release/app").is_file());
    assert!(!p.join("build/Release/Debug/app").exists());
}

#[test]
fn run_library_has_no_target() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "lib", "--type", "lib"]));

    let out = cppx(&tmp.path().join("lib"), &["run"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("no run target"));
}

#[test]
fn header_only_runs_scratch_test() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "hdr", "--type", "header"]));
    let p = tmp.path().join("hdr");

    assert!(p.join("include/main.hpp").is_file());
    let gitignore = fs::read_to_string(p.join(".gitignore")).unwrap();
    assert!(gitignore.contains("main.cpp"));

    let out = cppx(&p, &["run"]);
    assert_ok(&out);
    assert!(stdout(&out).contains("Hello from hdr"));
}

#[test]
fn header_only_adds_no_test_target_when_nested() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "parent"]));
    let parent = tmp.path().join("parent");
    assert_ok(&cppx(&parent, &["new", "hdr", "--type", "header"]));

    // parent has its own main.cpp at the root, which the old guard picked up
    fs::write(parent.join("main.cpp"), "int main() {}\n").unwrap();

    // a nested project can't re-set the compiler once the parent enabled the language
    let hdr_cmake = parent.join("hdr/CMakeLists.txt");
    let stripped: String = fs::read_to_string(&hdr_cmake)
        .unwrap()
        .lines()
        .filter(|l| !l.contains("CMAKE_CXX_COMPILER "))
        .map(|l| format!("{l}\n"))
        .collect();
    fs::write(&hdr_cmake, stripped).unwrap();

    let mut cmake = fs::read_to_string(parent.join("CMakeLists.txt")).unwrap();
    cmake.push_str("add_subdirectory(hdr)\n");
    fs::write(parent.join("CMakeLists.txt"), cmake).unwrap();

    assert_ok(&cppx(&parent, &["build"]));

    let help = Command::new("cmake")
        .args(["--build", "build/Debug", "--target", "help"])
        .current_dir(&parent)
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&help.stdout).contains("hdr_test"));
}

#[test]
fn clean_respects_build_dir_config() {
    let tmp = tempfile::tempdir().unwrap();
    assert_ok(&cppx(tmp.path(), &["new", "app"]));
    let p = tmp.path().join("app");

    fs::write(
        p.join(".cppx.json"),
        r#"{ "run-target": "app", "build-dir": "out" }"#,
    )
    .unwrap();

    assert_ok(&cppx(&p, &["build"]));
    assert!(p.join("out/Debug").is_dir());

    assert_ok(&cppx(&p, &["clean"]));
    assert!(!p.join("out").exists());

    // nothing left to clean
    assert!(!cppx(&p, &["clean"]).status.success());
}
