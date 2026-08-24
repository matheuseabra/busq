class Busq < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/busq"
  url "https://github.com/matheuseabra/busq/archive/refs/tags/v1.0.0.tar.gz"
  sha256 "REPLACE_AFTER_RELEASE"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "busq", shell_output("#{bin}/busq --version")
  end
end
