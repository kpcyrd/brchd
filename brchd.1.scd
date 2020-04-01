brchd(1)

# NAME

brchd - data exfiltration toolkit

# SYNOPSIS

*brchd* [options] [paths ...]

# RECEIVER

The receiver is the http server that accepts uploads and stores them inside a folder.

```
brchd -Hd drop/
```

It is recommended to use brchd to upload, but you can also open it in the
browser to use the html form.

There are multiple ways to use it with curl, the easiest being:

```
curl -F file=@README.md http://127.1:7070
```

To upload multiple files you can combine curl with tar:

```
tar cvvJ files/*.bin.part | curl -T- http://127.1:7070
```

To upload log files only readable by root:

```
sudo tar cvvJ /var/log/nginx | curl -T- http://127.1:7070
```

# DAEMON

The daemon runs on the system that is uploading.

```
brchd -Dd http://127.1:7070
```

It keeps track of the upload queue and restricts the number of concurrent
uploads.

# CLIENT

The client is the command you're going to use the most. To add files or
directories to the queue run:

```
brchd passwords.txt
brchd imgs/
brchd /var/log/*.log
```

You can attach a status monitor to watch the current status:

```
brchd
```

You can exit the monitor with *^C* at any time without interrupting any
uploads.

For scripting you may want to use *brchd -w* to block the script until all
uploads are done.

# CONFIGURATION

You can create a configuration file to define defaults. The system
configuration is only going to be loaded if the user didn't create one, and
*-c* is not used.

[[ *Operating System*
:- _System_
:- _User_
|  *Linux*
:  /etc/brchd.toml
:  ~/.config/brchd.toml
|  *Mac OSX*
:  /etc/brchd.toml
:  ~/Library/Preferences/brchd.toml
|  *Windows*
:  ???
:  %APPDATA%/brchd.toml


Everything is optional, but a minimal configuration might look like this:

```
[daemon]
destination = "http://127.0.0.1:7070"

[http]
destination = "/home/user/drop"
```

Usually you wouldn't configure both sections on the same system, but it works
for demoing purpose.

# ENCRYPTION

brchd supports encrypting files with asymetric encryption to hide the file
content from the server you're uploading to.

The header starts with the magic bytes *\\x00#BRCHD\\x00*, a 24 byte nonce, a
32 byte disposable public key, and a u16 that indicates the length of the
header body. The header body is encrypted with
*crypto_box_curve25519xsalsa20poly1305* and contains the symetric encryption
key for the file and an optional filename.

This header is then followed by 24 header bytes and 4096 byte chunks that are
encrypted with *crypto_secretstream_xchacha20poly1305*. When decrypting you
need to read in 4096+17 byte chunks due to the authenticator.

To get started you first need to generate a keypair:

```
$ brchd --keygen
[crypto]
#pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
seckey = "5LYdSbVM3Pxnvzi71bZedjNXgnu0ZIjEObJeTqa3UAU="
```

Encrypting on-the-fly is not available yet. You can encrypt individual files or
folders with:

```
brchd --encrypt --pubkey $PUBKEY file.txt
```

The *--pubkey* flag is not required if you declare your public key in your
config file:

```
[crypto]
pubkey = "your key here"
```

To decrypt everything in your upload folder, run:

```
brchd --decrypt drop/
```

It is *highly recommended* to configure the secret key in the config file or as
an environment variable for security reasons.

# PATHSPEC

You can configure how uploads are going to be stored with the *-F* flag.

By default brchd is going to refuse to overwrite files, to avoid having any
uploads denied you may use *%r-%f* to have everything prefixed with a random
string.

You may also use a folder structure based on the upload date with
*%Y/%m/%d/%p*.

[[ *Variable*
:- _Value_
|  *%%*
:  Literal *%*
|  *%Y*
:  Current year
|  *%m*
:  Current month
|  *%d*
:  Current day
|  *%H*
:  Current hour
|  *%M*
:  Current minute
|  *%S*
:  Current second
|  *%h*
:  Remote IP
|  *%f*
:  Filename
|  *%p*
:  Path
|  *%P*
:  Absolute path (if available, falls back to %p)


# AUTHORS

This program was originally written and is currently maintained by kpcyrd. Bug
reports and patches are welcome on github: https://github.com/kpcyrd/brchd
