class Scafalra < Formula
  desc "scafalra(sca) is a command-line interface tool for managing templates"
  homepage "https://github.com/shixinhuang99/scafalra"
  license "MIT"

  version "1.4.1"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/shixinhuang99/scafalra/releases/download/#{version}/sca-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "sha256:1a292ce521bbd843b4c67c53d7d0dbf61badfae82250a33479af49c5a8d128a9"
    else
      url "https://github.com/shixinhuang99/scafalra/releases/download/#{version}/sca-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "sha256:dcf719fd3f18cfc2120848b2b72031dd585b714b2ced505c3c565048ae6f1a7c"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/shixinhuang99/scafalra/releases/download/#{version}/sca-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "sha256:db8ca81ada7458b7031fc40f502ce0ccfb8ec282bf86c3b0fe7ebb8144477abe"
    else
      url "https://github.com/shixinhuang99/scafalra/releases/download/#{version}/sca-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "sha256:54593fbe5b1ed6e2e371569a0eb8257b86b491e66fc95cc5f3abd1fe7ef68a48"
    end
  end

  def install
    bin.install "sca"
  end

  test do
    system "#{bin}/sca", "--version"
  end
end
