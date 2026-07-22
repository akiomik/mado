# frozen_string_literal: true

class Mado < Formula
  desc "Fast Markdown linter written in Rust"
  homepage "https://github.com/akiomik/mado"
  version "0.3.1"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/akiomik/mado/releases/download/v#{version}/mado-macOS-arm64.tar.gz"
      sha256 "8043af62e1d2f726b34cb34c9f66fe74f9bc1750666351872318fb680f20cde6"
    end

    on_intel do
      url "https://github.com/akiomik/mado/releases/download/v#{version}/mado-macOS-x86_64.tar.gz"
      sha256 "c7d0cc6665ca9d6535219a790222913a6d92d7a170aa4e7665cec4c2a1427646"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/akiomik/mado/releases/download/v#{version}/mado-Linux-gnu-arm64.tar.gz"
      sha256 "cb7df9feff1c117eeedc7b33267d853980192b7cffceb723794f74d54cde1522"
    end

    on_intel do
      url "https://github.com/akiomik/mado/releases/download/v#{version}/mado-Linux-gnu-x86_64.tar.gz"
      sha256 "403f72360876d42301b79f879344ba27bf320ca885069db2c697cd21acaf1f4d"
    end
  end

  def install
    bin.install "mado"
  end

  test do
    assert_equal "mado #{version}", shell_output("#{bin}/mado --version").strip
  end
end
