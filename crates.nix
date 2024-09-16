{...}: {
  perSystem = {
    pkgs,
    config,
    ...
  }: let
    crateName = "quartz";
  in {
    nci.toolchainConfig = ./rust-toolchain.toml;
    # declare projects
    nci.projects."quartz".path = ./.;
    # configure crates
    nci.crates.${crateName} = {};
  };
}
