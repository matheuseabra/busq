class Minfetch < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/minfetch"
  url "https://github.com/matheuseabra/minfetch/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "055735041f4ab9fff0c5a47e806c280baa9e23339a1889d3a0e418e5393f48b2"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "minfetch", shell_output("#{bin}/minfetch --version")
  end
end
