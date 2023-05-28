{
  description = "Development environment for this project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, nixpkgs-unstable, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        mkOverlay = input: name: (final: prev: {
          "${name}" = import input {
            system = final.system;
            config = final.config;
          };
        });
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (mkOverlay nixpkgs-unstable "unstable")
          ];
        };
      in
      rec {
        devShells = {
          default =
            let
              lint = with pkgs; [ formatter rustfmt ];
              ls = with pkgs;[ nil rust-analyzer ];
              deps = with pkgs; ([ cargo rustc cargo-edit ] ++ (lib.optional pkgs.stdenvNoCC.isDarwin [ darwin.apple_sdk.frameworks.Security libiconv ]));
              tools = with pkgs; [ tcpdump termshark ];
            in
            pkgs.mkShell
              {
                name = "crash";

                packages = lint ++ ls ++ deps ++ tools;
              };
        };
        formatter = pkgs.nixpkgs-fmt;
      });
}
