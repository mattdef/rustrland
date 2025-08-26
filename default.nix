{
  lib,
  rustPlatform,
  fetchFromGitHub,
  versionCheckHook,
  nix-update-script,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "rustrland";
  version = "0.3.8";

  src = fetchFromGitHub {
    owner = "mattdef";
    repo = "rustrland";
    tag = "v${finalAttrs.version}";
    sha256 = "0f16bcb678ec9be54211b4c6810bf10b7a4bb7838f3af7a71a28b14da23c94d1";
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  postPatch = ''
    cp ${./Cargo.lock} Cargo.lock
  '';

  # Skip building examples that have compilation issues
  cargoBuildFlags = [ "--bins" ];
  cargoTestFlags = [ "--bins" ];

  doInstallCheck = true;
  nativeInstallCheckInputs = [ versionCheckHook ];

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "Rust-powered window management for Hyprland";
    homepage = "https://github.com/mattdef/rustrland";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ mattdef ];
    mainProgram = "rustrland";
  };
})