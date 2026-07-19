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
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          cargo-llvm-cov
          clang
          clippy
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
          (writeShellApplication {
            name = "ops";
            runtimeInputs = [ python3 ];
            text = ''
              exec python -m tools.pokemon_ops "$@"
            '';
          })
        ] ++ nativeLibraries;

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath nativeLibraries;
        RUST_BACKTRACE = "1";
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

        shellHook = ''
          export CC=clang
          export CXX=clang++
          export LLVM_COV="${pkgs.llvmPackages.llvm}/bin/llvm-cov"
          export LLVM_PROFDATA="${pkgs.llvmPackages.llvm}/bin/llvm-profdata"
        '';
      };
    };
}
