# brchd

## Starting the receiver somewhere

     brchd -Hd drop/

## Run the background uploader

    brchd -vDd http://127.0.0.1:7070

## Manage uploads

    brchd passwords.txt
    brchd imgs/
    brchd /var/log/*.log
    brchd # attaches status monitor
    brchd --wait # blocks until all pending uploads are done

## Install dependencies

    apt install pkg-config libsodium-dev
    pacman -S pkg-config libsodium
