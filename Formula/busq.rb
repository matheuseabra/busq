class Busq < Formula
  desc "Tiny, pane-aware system-info readout for terminals"
  homepage "https://github.com/matheuseabra/busq"
  url "https://github.com/matheuseabra/busq/archive/refs/tags/v1.1.0.tar.gz"
  sha256 "c4d51a205956a459b933dff0f959bdc46a9eaac6f64aa7f8748914fe3dc202b8"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "busq", shell_output("#{bin}/busq --version")
  end
end
