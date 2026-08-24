class Minfetch < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/minfetch"
  url "https://github.com/matheuseabra/minfetch/archive/refs/tags/v0.4.1.tar.gz"
  sha256 "ef3eee30f6e1b36a0014ec365573759f5c709fcae0211eb6d4242e18769568c0"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "minfetch", shell_output("#{bin}/minfetch --version")
  end
end
