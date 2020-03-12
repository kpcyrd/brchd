# brchd

## Starting the receiver somewhere

     brchd -Hp ./drop/ -B :1337

## Run the background uploader

    brchd -d

## Manage uploads

    brchd passwords.txt
    brchd imgs/
    brchd /var/log/*.log
    brchd # attaches status monitor
    brchd --wait # blocks until all pending uploads are done
