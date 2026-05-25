# typed: false
# frozen_string_literal: true

# Legacy compatibility template for the release workflow.
# Normal users should install `randlee/tap/sc-lint`; the update-homebrew job
# rewrites the tap checkout with release-pinned URLs and SHA256 values.
class ScLintBoundary < Formula
  desc "Legacy compatibility formula for the sc-lint boundary analyzer"
  homepage "https://github.com/randlee/sc-lint"
  version "0.0.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/randlee/sc-lint/releases/download/v0.0.0/sc-lint_0.0.0_x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"

      def install
        bin.install "sc-lint-boundary"
      end
    end
    on_arm do
      url "https://github.com/randlee/sc-lint/releases/download/v0.0.0/sc-lint_0.0.0_aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"

      def install
        bin.install "sc-lint-boundary"
      end
    end
  end

  on_linux do
    on_intel do
      if Hardware::CPU.is_64_bit?
        url "https://github.com/randlee/sc-lint/releases/download/v0.0.0/sc-lint_0.0.0_x86_64-unknown-linux-gnu.tar.gz"
        sha256 "0000000000000000000000000000000000000000000000000000000000000000"

        def install
          bin.install "sc-lint-boundary"
        end
      end
    end
  end

  test do
    system "#{bin}/sc-lint-boundary", "--version"
  end
end
