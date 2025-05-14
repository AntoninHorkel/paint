{
    description = "GPU accelerated paint app in Rust";
    inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    outputs = { self, nixpkgs }:
    let
        platforms = with nixpkgs.lib.platforms; linux ++ windows ++ darwin ++ freebsd;
        eachSystem = nixpkgs.lib.genAttrs platforms;
    in
    {
        packages = eachSystem (system:
        let
            pkgs = import nixpkgs { inherit system; };
            lib = pkgs.lib;
            cargo_toml = lib.importTOML ./Cargo.toml;
        in
        {
            default = pkgs.rustPlatform.buildRustPackage {
                inherit (cargo_toml.package) version;
                pname = cargo_toml.package.name;
                src = lib.cleanSourceWith {
                    src = ./.;
                    filter = path: type: !(lib.elem path cargo_toml.package.exclude ++ ".git/");
                };
                cargoLock = { lockFile = ./Cargo.lock; };
                buildInputs = with pkgs; [
                    libGL
                    libxkbcommon
                    # vulkan-loader
                    # vulkan-validation-layers
                    wayland
                ];
                meta = {
                    inherit (cargo_toml.package) description;
                    inherit platforms;
                    homepage = cargo_toml.package.repository;
                    license = with lib.licenses; [ mit apache2 ];
                    mainProgram = "paint";
                };
            };
        });
        devShells = eachSystem (system:
        let
            pkgs = import nixpkgs { inherit system; };
        in
        {
            default = import ./shell.nix { inherit pkgs; };
        });
    };
}
