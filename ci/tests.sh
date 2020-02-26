#!/bin/sh

# working
curl -sS \
    -H 'Content-Type: multipart/form-data; boundary=------------------------457a7852da6997ff' \
    http://127.0.0.1:3000/ \
    --data-binary $'--------------------------457a7852da6997ff\r
Content-Disposition: form-data; name="foo"; filename="ohai.txt"\r
\r
yey!\r
\r
--------------------------457a7852da6997ff--\r
'

# overwrite the file
curl -sS \
    -H 'Content-Type: multipart/form-data; boundary=------------------------457a7852da6997ff' \
    http://127.0.0.1:3000/ \
    --data-binary $'--------------------------457a7852da6997ff\r
Content-Disposition: form-data; name="foo"; filename="ohai.txt"\r
\r
pwned!\r
\r
--------------------------457a7852da6997ff--\r
'

# absolute path
curl -sS \
    -H 'Content-Type: multipart/form-data; boundary=------------------------457a7852da6997ff' \
    http://127.0.0.1:3000/ \
    --data-binary $'--------------------------457a7852da6997ff\r
Content-Disposition: form-data; name="foo"; filename="/tmp/x"\r
\r
bar\r
\r
--------------------------457a7852da6997ff--\r
'

# traversal
curl -sS \
    -H 'Content-Type: multipart/form-data; boundary=------------------------457a7852da6997ff' \
    http://127.0.0.1:3000/ \
    --data-binary $'--------------------------457a7852da6997ff\r
Content-Disposition: form-data; name="foo"; filename="../x"\r
\r
bar\r
\r
--------------------------457a7852da6997ff--\r
'

# subfolder
curl -sS \
    -H 'Content-Type: multipart/form-data; boundary=------------------------457a7852da6997ff' \
    http://127.0.0.1:3000/ \
    --data-binary $'--------------------------457a7852da6997ff\r
Content-Disposition: form-data; name="foo"; filename="a/b/c.txt"\r
\r
foo\r
\r
--------------------------457a7852da6997ff--\r
'
curl -sS \
    -H 'Content-Type: multipart/form-data; boundary=------------------------457a7852da6997ff' \
    http://127.0.0.1:3000/ \
    --data-binary $'--------------------------457a7852da6997ff\r
Content-Disposition: form-data; name="foo"; filename="a/b/d.txt"\r
\r
bar\r
\r
--------------------------457a7852da6997ff--\r
'
