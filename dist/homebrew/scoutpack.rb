class Scoutpack < Formula
  desc "Offline repo context compiler for AI coding agents"
  homepage "https://github.com/theamodhshetty/ScoutPack"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/theamodhshetty/ScoutPack/releases/download/v#{version}/scoutpack-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_AARCH64_APPLE_DARWIN_SHA256"
    else
      url "https://github.com/theamodhshetty/ScoutPack/releases/download/v#{version}/scoutpack-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_X86_64_APPLE_DARWIN_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/theamodhshetty/ScoutPack/releases/download/v#{version}/scoutpack-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_AARCH64_LINUX_SHA256"
    else
      url "https://github.com/theamodhshetty/ScoutPack/releases/download/v#{version}/scoutpack-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_X86_64_LINUX_SHA256"
    end
  end

  def install
    bin.install "scoutpack"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/scoutpack --version")
  end
end
