{ pkgs, lib, config, inputs, ... }:

{
  env.CGO_ENABLED = 1;
  packages = [ pkgs.git ];
  languages.rust.enable = true;
  languages.go.enable = true;
}
