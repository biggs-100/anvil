# typed: true
# frozen_string_literal: true

# Anvil — reproducible development environments for humans, tools, and AI agents.
#
# Usage:
#   brew tap biggs-100/tap
#   brew install anvil
#
# Or directly from the formula file:
#   brew install --formula contrib/homebrew/anvil.rb

class Anvil < Formula
  desc "Reproducible development environments for humans, tools, and AI agents"
  homepage "https://github.com/biggs-100/anvil"
  license "MIT"
  head "https://github.com/biggs-100/anvil.git", branch: "main"

  stable do
    url "https://github.com/biggs-100/anvil.git",
        tag:      "v0.1.0",
        revision: "6e850fb" # FIXME: update on release

    # Binary release (preferred for end-users)
    # Uncomment and set the correct SHA256 when publishing a release:
    # on_macos do
    #   if Hardware::CPU.arm?
    #     url "https://github.com/biggs-100/anvil/releases/download/v0.1.0/anvil-aarch64-apple-darwin.tar.gz"
    #     sha256 "..."
    #   else
    #     url "https://github.com/biggs-100/anvil/releases/download/v0.1.0/anvil-x86_64-apple-darwin.tar.gz"
    #     sha256 "..."
    #   end
    # end
    #
    # on_linux do
    #   if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
    #     url "https://github.com/biggs-100/anvil/releases/download/v0.1.0/anvil-aarch64-unknown-linux-gnu.tar.gz"
    #     sha256 "..."
    #   else
    #     url "https://github.com/biggs-100/anvil/releases/download/v0.1.0/anvil-x86_64-unknown-linux-gnu.tar.gz"
    #     sha256 "..."
    #   end
    # end
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/anvil-cli")
    # Symlink `anvil` without the `-cli` suffix for convenience
    bin.install_symlink "anvil-cli" => "anvil" unless build.head?
  end

  test do
    assert_match "anvil", shell_output("#{bin}/anvil --version")
  end
end
