{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "rust-analyzer" ];
        targets = [ "x86_64-unknown-linux-musl" ];
      };
    in {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          rust
          pkgs.pkg-config
          pkgs.bore-cli
          pkgs.pkgsStatic.stdenv.cc
        ];
        # For musl static builds: cargo build --release --target x86_64-unknown-linux-musl
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${pkgs.pkgsStatic.stdenv.cc}/bin/x86_64-unknown-linux-musl-cc";
        CC_x86_64_unknown_linux_musl = "${pkgs.pkgsStatic.stdenv.cc}/bin/x86_64-unknown-linux-musl-cc";
      };
    };
}
