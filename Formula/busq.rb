class Busq < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/busq"
  url "https://github.com/matheuseabra/busq/archive/refs/tags/v1.2.0.tar.gz"
  sha256 "8f81c5f9baf7c6ac6851476cc450242cfe11f991c968ac0f456103b6437eeb83"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "busq", shell_output("#{bin}/busq --version")
  end
end
