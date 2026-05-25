# typed: false
# frozen_string_literal: true

# Bootstrap template for the release workflow.
# The update-homebrew job rewrites the tap checkout with release-pinned URLs
# and SHA256 values after each published sc-lint release.
class ScLint < Formula
  desc "Top-level sc-lint CLI and analyzer toolset for Rust workspaces"
  homepage "https://github.com/randlee/sc-lint"
  version "0.0.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/randlee/sc-lint/releases/download/v0.0.0/sc-lint_0.0.0_x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"

      def install
        bin.install "sc-lint"
        bin.install "sc-lint-boundary"
        bin.install "sc-lint-portability"
        bin.install "sc-lint-runtime"
      end
    end
    on_arm do
      url "https://github.com/randlee/sc-lint/releases/download/v0.0.0/sc-lint_0.0.0_aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"

      def install
        bin.install "sc-lint"
        bin.install "sc-lint-boundary"
        bin.install "sc-lint-portability"
        bin.install "sc-lint-runtime"
      end
    end
  end

  on_linux do
    on_intel do
      if Hardware::CPU.is_64_bit?
        url "https://github.com/randlee/sc-lint/releases/download/v0.0.0/sc-lint_0.0.0_x86_64-unknown-linux-gnu.tar.gz"
        sha256 "0000000000000000000000000000000000000000000000000000000000000000"

        def install
          bin.install "sc-lint"
          bin.install "sc-lint-boundary"
          bin.install "sc-lint-portability"
          bin.install "sc-lint-runtime"
        end
      end
    end
  end

  test do
    system "#{bin}/sc-lint", "--version"
    system "#{bin}/sc-lint-boundary", "--version"
    system "#{bin}/sc-lint-portability", "--version"
    system "#{bin}/sc-lint-runtime", "--version"
  end
end
