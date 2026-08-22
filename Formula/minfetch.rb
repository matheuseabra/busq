class Minfetch < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/minfetch"
  license "MIT"
  head "https://github.com/matheuseabra/minfetch.git", branch: "phase2-layout-roadmap"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "minfetch", shell_output("#{bin}/minfetch --version")
  end
end
