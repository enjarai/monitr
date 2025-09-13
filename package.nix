{ lib
, rustPlatform
, openssl
, alsa-lib
, pkgconf }:

rustPlatform.buildRustPackage rec {
  pname = "monitr";
  version = "0.1.0";

  src = ./.;

  postInstall = ''
    cp -r static $out/static
  '';

  buildInputs = [
    openssl
  ];

  nativeBuildInputs = [
    pkgconf
  ];

  cargoLock.lockFile = src + /Cargo.lock;
  doCheck = false;

  meta = with lib; {
    homepage = "https://github.com/enjarai/monitr";
    description = "";
    #TODO: decide on the license
    # license = licenses.mit;
  };
}
