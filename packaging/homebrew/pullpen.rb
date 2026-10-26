# Homebrew formula template for a tap (`brew install <tap>/pullpen`).
# VERSION and the four SHA256 values come from a release's checksums.txt;
# the tap repository that serves this file does not exist yet.
class Pullpen < Formula
  desc "Review, arm and merge pull requests across GitHub and GitLab from one terminal queue"
  homepage "https://github.com/bspeelm/pullpen"
  version "VERSION"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/bspeelm/pullpen/releases/download/vVERSION/pullpen_darwin_arm64.tar.gz"
      sha256 "SHA256_DARWIN_ARM64"
    end
    on_intel do
      url "https://github.com/bspeelm/pullpen/releases/download/vVERSION/pullpen_darwin_amd64.tar.gz"
      sha256 "SHA256_DARWIN_AMD64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/bspeelm/pullpen/releases/download/vVERSION/pullpen_linux_arm64.tar.gz"
      sha256 "SHA256_LINUX_ARM64"
    end
    on_intel do
      url "https://github.com/bspeelm/pullpen/releases/download/vVERSION/pullpen_linux_amd64.tar.gz"
      sha256 "SHA256_LINUX_AMD64"
    end
  end

  def install
    bin.install "pullpen"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/pullpen --version")
  end
end
