set shell := ["bash", "-euo", "pipefail", "-c"]

build:
  @if [[ ! -f main.typ ]]; then echo "main.typ does not exist yet; nothing to build."; exit 0; fi
  mkdir -p build
  typst compile main.typ build/slides.pdf

watch:
  @if [[ ! -f main.typ ]]; then echo "main.typ does not exist yet; cannot watch." >&2; exit 1; fi
  mkdir -p build
  typst watch main.typ build/slides.pdf

clean:
  rm -rf build

assets:
  python3 scripts/render_radar.py
  python3 scripts/render_architecture.py

fonts:
  @fonts="$(typst fonts)"; for family in "New Computer Modern Mono" "New Computer Modern" "New Computer Modern Sans" "New Computer Modern Math" "Source Han Serif SC" "Sarasa UI SC"; do grep -Fqx "$family" <<<"$fonts" || { echo "Missing Typst font family: $family" >&2; exit 1; }; echo "found: $family"; done

check: fonts assets

render: build
  @if [[ ! -f build/slides.pdf ]]; then echo "build/slides.pdf does not exist; nothing to render."; exit 0; fi
  mkdir -p build/rendered
  pdftoppm -png -r 144 build/slides.pdf build/rendered/slide
