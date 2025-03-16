#! /bin/bash

names=("regisd" "regisc" "regis");
source_dir="./compiled/"
dest_dir="/usr/bin/"

for name in "${names[@]}"; do 
    cp "$source_dir$name" "$dest_dir$name"
    loc="$dest_dir$name"
done

# Clean out old files
rm -rf /etc/regis/
# Install a new directory
mkdir -p /etc/regis/

cp ./regisd.service /lib/systemd/system
cp ./stress-ng.service /lib/systemd/system

# install needed scripts
cp ./bundle/* /etc/regis/