class Minfetch < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/minfetch"
  url "https://github.com/matheuseabra/minfetch/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "bede4cb12f187d4bd15da356d5aa1fbfaf606c3e0f933edf219542263997d04d"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "minfetch", shell_output("#{bin}/minfetch --version")
  end
end
