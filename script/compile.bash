#! /bin/bash

names=("regisc" "regisd" "regisg")
dest="./compiled/"

for name in "${names[@]}"; do
    cargo build --release --manifest-path "../$name/Cargo.toml" 
done

for name in "${names[@]}"; do 
    cp "../$name/target/release/$name" $dest
done
for name in "${names[@]}"; do
    chmod +x "./compiled/$name"
done