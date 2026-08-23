class Minfetch < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/minfetch"
  url "https://github.com/matheuseabra/minfetch/archive/refs/tags/v0.4.0.tar.gz"
  sha256 "07aa3de5d8c25a23b566c41ee22a0760a7a5104c49a0100ebb9ccf064e439e0f"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "minfetch", shell_output("#{bin}/minfetch --version")
  end
end
