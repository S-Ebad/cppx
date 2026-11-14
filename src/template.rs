pub const CLANGD: &str = r#"
CompileFlags:
    Add:
        - -std={STD}
        - -Wall
        - -Wextra
"#;

pub const CMAKE_EXE: &str = r#"
cmake_minimum_required(VERSION {CMAKE_MIN})
set(CMAKE_{LANG}_STANDARD {STD_NUM})
set(CMAKE_{LANG}_STANDARD_REQUIRED ON)
set(CMAKE_{LANG}_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

project({PROJECT} LANGUAGES {LANG})

file(GLOB_RECURSE SOURCES CONFIGURE_DEPENDS
    "src/*{LANG_EXT}"
)

add_executable(${{PROJECT_NAME}} ${{SOURCES}})

target_include_directories(${{PROJECT_NAME}} PRIVATE
    include
)

if (CMAKE_{LANG}_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(${{PROJECT_NAME}} PRIVATE
        -Wall
        -Wextra
        -Wpedantic
    )
endif()
"#;

pub const CMAKE_HEADER: &str = r#"
cmake_minimum_required(VERSION {CMAKE_MIN})
set(CMAKE_{LANG}_STANDARD {STD_NUM})
set(CMAKE_{LANG}_STANDARD_REQUIRED ON)
set(CMAKE_{LANG}_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

project({PROJECT} LANGUAGES {LANG})

add_library(${{PROJECT_NAME}} INTERFACE)
target_include_directories(${{PROJECT_NAME}} INTERFACE
    include
)

# only when built directly, so a parent's add_subdirectory never picks up a stray test target
if (CMAKE_CURRENT_SOURCE_DIR STREQUAL CMAKE_SOURCE_DIR AND EXISTS "${{CMAKE_CURRENT_SOURCE_DIR}}/main{LANG_EXT}")
    add_executable(${{PROJECT_NAME}}_test main{LANG_EXT})
    target_link_libraries(${{PROJECT_NAME}}_test PRIVATE ${{PROJECT_NAME}})

    if (CMAKE_{LANG}_COMPILER_ID MATCHES "Clang|GNU")
        target_compile_options(${{PROJECT_NAME}}_test PRIVATE
            -Wall
            -Wextra
            -Wpedantic
        )
    endif()
endif()
"#;

pub const CMAKE_LIB: &str = r#"
cmake_minimum_required(VERSION {CMAKE_MIN})
set(CMAKE_{LANG}_STANDARD {STD_NUM})
set(CMAKE_{LANG}_STANDARD_REQUIRED ON)
set(CMAKE_{LANG}_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

project({PROJECT} LANGUAGES {LANG})

file(GLOB_RECURSE SOURCES CONFIGURE_DEPENDS
    "src/*{LANG_EXT}"
)

add_library(${{PROJECT_NAME}} STATIC ${{SOURCES}})

target_include_directories(${{PROJECT_NAME}} PUBLIC
    include
)

if (CMAKE_{LANG}_COMPILER_ID MATCHES "Clang|GNU")
    target_compile_options(${{PROJECT_NAME}} PRIVATE
        -Wall
        -Wextra
        -Wpedantic
    )
endif()
"#;

pub const MAIN_CPP: &str = r#"#include <iostream>
int main() {{
  std::cout << "Hello from {PROJECT_IDENT}" << std::endl;
}}"#;

pub const MAIN_C: &str = r#"#include <stdio.h>

int main(void) {{
    printf("Hello from {PROJECT_IDENT}\n");
}}
"#;

pub const LIB_CPP: &str = r#"
#include <iostream>

namespace {PROJECT_IDENT} {{
void main() {{
    std::cout << "Hello from {PROJECT_IDENT}";
}}
}}
"#;

pub const LIB_C: &str = r#"
#include <stdio.h>

void {PROJECT_IDENT}_main() {{
    printf("Hello from {PROJECT_IDENT}");
}}
"#;

pub const MAIN_HPP: &str = r#"#pragma once
#include <iostream>

namespace {PROJECT_IDENT} {{

inline void hello() {{
    // TODO: lib code
}}

}} // namespace {PROJECT_IDENT}
"#;

pub const MAIN_H: &str = r#"
#ifndef {PROJECT_GUARD}_H
#define {PROJECT_GUARD}_H
#include <stdio.h>

void {PROJECT_IDENT}_hello(void);

#ifdef {PROJECT_GUARD}_IMPLEMENTATION
void {PROJECT_IDENT}_hello(void) {{
    // TODO: lib code
}}
#endif // {PROJECT_GUARD}_IMPLEMENTATION
#endif // {PROJECT_GUARD}_H
"#;

pub const GITIGNORE: &str = r#"
# Build
build/

# cache
.cache

# ide cruft
.vscode/
.idea/
*.swp

"#;
