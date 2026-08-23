class Minfetch < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/minfetch"
  url "https://github.com/matheuseabra/minfetch/archive/refs/tags/v0.3.0.tar.gz"
  sha256 "ef6e964a9ec580c8c76f93898653840137ea061244369ed961272fcac91139f9"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "minfetch", shell_output("#{bin}/minfetch --version")
  end
end
