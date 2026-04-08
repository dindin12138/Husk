{ pkgs, ... }:
{
  env.PROJECT_NAME = "Husk";

  languages.rust.enable = true;
  languages.zig.enable = true;

  packages = with pkgs; [
    pkg-config

    wayland
    wayland-protocols
    wayland-scanner
    libxkbcommon

    vulkan-loader
    libclang
  ];
  env.LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

  # The crucial fix for NixOS + winit/wgpu:
  # Instructs dlopen where to find shared libraries at runtime
  env.LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
    with pkgs;
    [
      wayland
      libxkbcommon
      vulkan-loader
    ]
  );

  enterShell = ''
    echo "===================================================="
    echo "🦀 Welcome to the $PROJECT_NAME (Rust) Dev Environment!"
    echo "===================================================="
    echo "🦀 Rust Toolchain : $(cargo --version | head -n 1)"
    echo "⚡ Zig Compiler   : $(zig version)"
    echo "===================================================="

    if [ -z "$WAYLAND_DISPLAY" ]; then
      echo "⚠️ Warning: WAYLAND_DISPLAY is not set. Are you running a Wayland compositor?"
    fi
  '';
}
