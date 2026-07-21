{
  description = "Pokemon Untitled Rust workspace development shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      nativeLibraries = with pkgs; [
        libGL
        libxkbcommon
        vulkan-loader
        wayland
        libx11
        libxcursor
        libxi
        libxrandr
      ];
      wslgRuntimeLibraries = with pkgs; [
        libxkbcommon
        wayland
      ];
      fontConfig = pkgs.makeFontsConf {
        fontDirectories = [ pkgs.noto-fonts-cjk-sans ];
      };
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          cargo-llvm-cov
          clang
          clippy
          fontconfig
          git
          git-lfs
          lld
          llvmPackages.llvm
          pkg-config
          python3
          rust-analyzer
          rustc
          rustfmt
          tokei
          uv
          noto-fonts-cjk-sans
          (writeShellApplication {
            name = "ops";
            runtimeInputs = [ python3 ];
            text = ''
              exec python -m tools.pokemon_ops "$@"
            '';
          })
        ] ++ nativeLibraries;

        RUST_BACKTRACE = "1";
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        FONTCONFIG_FILE = fontConfig;

        shellHook = ''
          if [ "''${WSL2_GUI_APPS_ENABLED:-}" = "1" ]; then
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath wslgRuntimeLibraries}"
          fi
          export CC=clang
          export CXX=clang++
          export LLVM_COV="${pkgs.llvmPackages.llvm}/bin/llvm-cov"
          export LLVM_PROFDATA="${pkgs.llvmPackages.llvm}/bin/llvm-profdata"
        '';
      };
    };
}
