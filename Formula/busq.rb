class Busq < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/busq"
  url "https://github.com/matheuseabra/busq/archive/refs/tags/v1.2.0.tar.gz"
  sha256 "343340cd662d2dbc46e5efb05ee1b82b5ed2380ed5a2539812fe768c5e05afed"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "busq", shell_output("#{bin}/busq --version")
  end
end
