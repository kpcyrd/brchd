# Maintainer: kpcyrd <kpcyrd[at]archlinux[dot]org>

pkgname=brchd
pkgver=0.0.0
pkgrel=1
pkgdesc='Data exfiltration toolkit'
url='https://github.com/kpcyrd/brchd'
arch=('x86_64')
license=('GPL3')
depends=('libsodium')
makedepends=('cargo' 'scdoc')

build() {
  cd ..
  cargo build --release --locked
  scdoc < brchd.1.scd > brchd.1
}

package() {
  cd ..
  install -Dm 755 target/release/${pkgname} -t "${pkgdir}/usr/bin"

  install -d "${pkgdir}/usr/share/bash-completion/completions" \
             "${pkgdir}/usr/share/zsh/site-functions" \
             "${pkgdir}/usr/share/fish/vendor_completions.d"
  "${pkgdir}/usr/bin/brchd" --gen-completions bash > "${pkgdir}/usr/share/bash-completion/completions/brchd"
  "${pkgdir}/usr/bin/brchd" --gen-completions zsh > "${pkgdir}/usr/share/zsh/site-functions/_brchd"
  "${pkgdir}/usr/bin/brchd" --gen-completions fish > "${pkgdir}/usr/share/fish/vendor_completions.d/brchd.fish"

  install -Dm 644 brchd.1 -t "${pkgdir}/usr/share/man/man1"
  install -Dm 644 README.md -t "${pkgdir}/usr/share/doc/${pkgname}"
}

# vim: ts=2 sw=2 et:
