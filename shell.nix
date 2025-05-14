{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShellNoCC {
    name = "paint";
    packages = with pkgs; [
        cargo
        clippy
        git
        lldb
        pre-commit
        rustc
        (rustfmt.override { asNightly = true; })
        rust-analyzer
        wgsl-analyzer
    ];
    shellHook = "pre-commit install";
    LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
        libGL
        libxkbcommon
        # vulkan-loader
        # vulkan-validation-layers
        wayland
    ];
    # VULKAN_SDK = pkgs.vulkan-loader;
    # VK_SDK_PATH = pkgs.vulkan-loader;
}
