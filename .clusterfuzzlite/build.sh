#!/bin/bash -eu
# Construit les cibles de fuzz/ pour ClusterFuzzLite (et OSS-Fuzz).

# Le serveur embarque web/build à la compilation ; les lecteurs fuzzés n'en
# ont pas besoin, une page vide suffit.
if [ ! -e web/build/index.html ]; then
  mkdir -p web/build
  echo '<!doctype html>' > web/build/index.html
fi

# Seul le code Rust est instrumenté. Les drapeaux C d'OSS-Fuzz (-O1, ASan)
# cassent la compilation d'aws-lc (son jitterentropy exige -O0) et le code C
# des dépendances n'est pas ce que l'on fuzze.
unset CFLAGS CXXFLAGS CFLAGS_x86_64_unknown_linux_gnu CXXFLAGS_x86_64_unknown_linux_gnu

cd fuzz
cargo fuzz build -O
for target in $(cargo fuzz list); do
  cp "target/x86_64-unknown-linux-gnu/release/$target" "$OUT/"
done
