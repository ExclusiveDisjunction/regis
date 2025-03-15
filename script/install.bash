#! /bin/bash

names=("regisd" "regisc" "regisg");
source_dir="./compiled/"
dest_dir="/usr/bin/"

for name in "${names[@]}"; do 
    cp "$source_dir$name" "$dest_dir$name"
    loc="$dest_dir$name"
    chown root:root $loc
    chmod 755 $loc 
done