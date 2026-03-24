cmake_minimum_required(VERSION 3.20)
project(Husk CXX C)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(FetchContent)

# --- Fetch libghostty-vt ---
message(STATUS "Fetching libghostty...")
FetchContent_Declare(
  ghostty
  GIT_REPOSITORY https://github.com/ghostty-org/ghostty.git
  GIT_TAG ecc55b94c803789762682065ab68f227447909c5)
FetchContent_MakeAvailable(ghostty)

# --- Find Wayland & EGL Dependencies ---
find_package(PkgConfig REQUIRED)
pkg_check_modules(WAYLAND REQUIRED wayland-client wayland-egl egl xkbcommon)

file(GLOB_RECURSE HUSK_SOURCES "src/*.cpp")

# --- Define Husk Executable ---
add_executable(${PROJECT_NAME} ${HUSK_SOURCES})

target_include_directories(
  ${PROJECT_NAME} PRIVATE ${WAYLAND_INCLUDE_DIRS}
                          "${CMAKE_CURRENT_SOURCE_DIR}/src")

target_link_libraries(${PROJECT_NAME} PRIVATE ghostty-vt ${WAYLAND_LIBRARIES})

# --- Hack: Patch libghostty headers for C++ compatibility ---
file(
  WRITE ${CMAKE_BINARY_DIR}/patch_ghostty.sh
  "find _deps/ghostty-src/zig-out/include/ghostty -type f -name '*.h' -exec sed -E -i 's/typedef struct ([A-Za-z0-9_]*) *\\* *\\1;/typedef struct \\1_s* \\1;/g' {} +\n"
)

add_custom_target(
  patch_ghostty_headers
  COMMAND bash patch_ghostty.sh
  WORKING_DIRECTORY ${CMAKE_BINARY_DIR}
  COMMENT "Patching libghostty C headers for C++ namespace compatibility...")

add_dependencies(patch_ghostty_headers ghostty-vt)
add_dependencies(${PROJECT_NAME} patch_ghostty_headers)
