{ pkgs, ... }:
{
  env.PROJECT_NAME = "Husk";

  languages.cplusplus.enable = true;
  languages.c.enable = true;
  languages.zig.enable = true;

  packages = with pkgs; [
    cmake
    ninja
    pkg-config
    cacert

    wayland
    wayland-protocols
    wayland-scanner
    libxkbcommon
    libGL
    egl-wayland
    libffi
    sdl3
    wgpu-native
  ];

  enterShell = ''
    echo "===================================================="
    echo " 🐚 Welcome to the $PROJECT_NAME Development Environment!"
    echo "===================================================="
    echo "🛠️ C++ Compiler : $(c++ --version | head -n 1)"
    echo "⚡ Zig Compiler : $(zig version)"
    echo "📦 CMake        : $(cmake --version | head -n 1)"
    echo "===================================================="

    if [ -z "$WAYLAND_DISPLAY" ]; then
      echo "⚠️ Warning: WAYLAND_DISPLAY is not set. Are you running a Wayland compositor?"
    fi
  '';
}
