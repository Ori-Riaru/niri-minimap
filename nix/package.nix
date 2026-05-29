{rustPlatform, ...}:
rustPlatform.buildRustPackage {
  name = "niri-minimap";
  src = ../.;
  cargoLock.lockFile = ../Cargo.lock;
}
