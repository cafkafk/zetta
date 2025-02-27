#!/usr/bin/env bash

if [ -z "$1" ]; then
    echo "Usage: $0 <output_dir>";
    exit 1;
fi

rm "$1" -rf;
mkdir -p "$1";
cd "$1" || exit;

sudo groupadd -f eza_test

# BEGIN grid
mkdir -p grid
cd grid || exit

mkdir $(seq -w 001 1000);
seq 0001 1000 | split -l 1 -a 3 -d - file_

# Set time to unix epoch
touch --date=@0 ./*;

cd .. || exit

# END grid

# BEGIN git

mkdir -p git
cd git || exit

mkdir $(seq -w 001 10);
for f in ./*
do
    cd "$f" || exit
    git init
    seq 01 10 | split -l 1 -a 3 -d - file_
    cd .. || exit
done

cd ..

# END git

# BEGIN test_root

sudo mkdir root
sudo chmod 777 root
sudo mkdir root/empty

# END test_root

# BEGIN mknod

mkdir -p specials

sudo mknod specials/block-device b  3 60
sudo mknod specials/char-device  c 14 40
sudo mknod specials/named-pipe   p

# END test_root

# BEGIN test_symlinks

mkdir -p symlinks
touch symlinks/file --date=@0
ln -s file symlinks/symlink
ln -s symlink symlinks/symlink2
mkdir -p symlinks/dir
ln -s dir symlinks/symlink3
ln -s pipitek symlinks/symlink4
touch "symlinks/ lorem ipsum" --date=@0
ln -s "lorem ipsum" "symlinks/ lorem ipsum"

# END test_symlinks

# BEGIN test_perms

mkdir -p perms
touch perms/file --date=@0
touch perms/file2 --date=@0
chmod 777 perms/file
chmod 001 perms/file2

# END test_perms

# BEGIN test_group
mkdir -p group
touch group/file --date=@0
sudo chgrp eza_test group/file
# END test_group

# BEGIN test_size
mkdir -p size
touch size/1M --date=@0
dd if=/dev/zero of=size/1M bs=1 count=0 seek=1M
touch size/1K --date=@0
dd if=/dev/zero of=size/1K bs=1 count=0 seek=1K
touch size/1B --date=@0
dd if=/dev/zero of=size/1B bs=1 count=0 seek=1
touch size/1337 --date=@0
dd if=/dev/zero of=size/1337 bs=1 count=0 seek=1337
# END test_size

# BEGIN test_time
mkdir -p time
touch time/epoch --date=@0
touch time/1s --date=@1
touch time/1m --date=@60
touch time/1h --date=@3600
touch time/1d --date=@86400
touch time/1y --date=@31536000
# END test_time

# BEGIN test_icons
mkdir -p icons
touch icons/file --date=@0
touch icons/go.go --date=@0
touch icons/rust.rs --date=@0
touch icons/c.c --date=@0
touch icons/c++.cpp --date=@0
touch icons/python.py --date=@0
touch icons/java.java --date=@0
touch icons/javascript.js --date=@0
touch icons/html.html --date=@0
touch icons/css.css --date=@0
touch icons/php.php --date=@0
touch icons/ruby.rb --date=@0
touch icons/shell.sh --date=@0
touch icons/unknown.unknown --date=@0
touch icons/man.1 --date=@0
touch icons/marked.md --date=@0
# END test_icons

# BEGIN submodule
cd git/*01 || exit
git add "*01"
# git doesn't allow us to create a submodule without a branch
GIT_COMMITTER_NAME="Your Name" GIT_COMMITTER_EMAIL="your@email.com" git commit --author="Your Name <your@email.com>" -m "Initial commit"
cd .. || exit

mkdir -p with_submodule
cd with_submodule || exit
git init
seq 01 10 | split -l 1 -a 3 -d - file_
git -c protocol.file.allow=always submodule add ../*01
touch --date=@0 ./*;
touch --date=@0 ./*01/*;
cd .. || exit
ln -s with_submodule with_submodule_symlink
cd .. || exit
# END submodule

# BEGIN set date
touch --date=@0 ./*;
# END set date
