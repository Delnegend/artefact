#!/usr/bin/env bash
# scripts/verify.sh — verify decoded output vs ffmpeg reference
# Catches 1x2/2x1 subsampling regressions like the 2x1 vertical shift bug (422/444 color bleed).
# Fails if mean/max/glitch exceed thresholds.
# Usage: ./scripts/verify.sh  (or: just verify)

set -euo pipefail

if ! command -v ffmpeg >/dev/null 2>&1; then
	echo "error: ffmpeg not found (needed for reference decode)" >&2
	exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
	echo "error: python3 not found" >&2
	exit 1
fi
python3 -c "import PIL" 2>&1 | grep -q "ModuleNotFoundError" && {
	echo "error: pillow not found — run: pip install --break-system-packages pillow" >&2
	exit 1
}

# ensure sample exists (generate-sample.sh now also encodes all 6 variants)
if [[ ! -f assets/sample.png ]]; then
	echo "assets/sample.png missing — generating via scripts/generate-sample.sh..."
	./scripts/generate-sample.sh assets/sample.png
fi

# ensure variants exist (in case sample.png was present but variants missing)
for s in 420 422 444; do
	if [[ ! -f assets/sample.$s.input.jpg ]]; then
		echo "assets/sample.$s.input.jpg missing — re-generating variants..."
		./scripts/generate-sample.sh assets/sample.png >/dev/null
		break
	fi
done

echo "Decoding via artefact-cli and ffmpeg reference..."
for s in 420 422 444; do
	cargo run --quiet --bin artefact-cli -- assets/sample.$s.input.jpg -o /tmp/verify.decoded.$s.png -y >/dev/null
	ffmpeg -hide_banner -loglevel error -y -i assets/sample.$s.input.jpg /tmp/verify.ref.$s.png
done

python3 << 'PY'
import sys
from PIL import Image

THRESHOLDS = {
	# tuned on 1600x1200 sample.png after fix (zune-jpeg mcu.rs:235 1x2 vertical)
	# 420: mean 2.7 max 78, 422: mean 2.6 max 70, 444: mean 2.5 max 69, glitch 0
	"mean_max": 10.0,
	"max_max": 100,
	"glitch_max": 0,
}

def check_one(s):
	ref = Image.open(f"/tmp/verify.ref.{s}.png").convert("RGB")
	dec = Image.open(f"/tmp/verify.decoded.{s}.png").convert("RGB")
	w, h = ref.size
	total = 0
	maxd = 0
	glitch = 0
	for y in range(0, h, 8):
		for x in range(0, w, 8):
			r = ref.getpixel((x, y))
			d = dec.getpixel((x, y))
			for j in range(3):
				diff = abs(r[j] - d[j])
				total += diff
				if diff > maxd:
					maxd = diff
				if diff > 100:
					glitch += 1
	sampled = (w * h // 64) * 3
	mean = total / sampled if sampled else 0

	xs = [145, 406, 667, 928, 1189, 1450]
	block_ok = True
	for x in xs:
		r = ref.getpixel((x, 500))
		d = dec.getpixel((x, 500))
		if max(abs(a - b) for a, b in zip(r, d)) > 30:
			block_ok = False

	checker_r = ref.getpixel((1340, 775))
	checker_d = dec.getpixel((1340, 775))
	checker_ok = max(abs(a - b) for a, b in zip(checker_r, checker_d)) < 40

	ok = True
	if mean > THRESHOLDS["mean_max"]:
		print(f"FAIL {s}: mean {mean:.2f} > {THRESHOLDS['mean_max']}")
		ok = False
	if maxd > THRESHOLDS["max_max"]:
		print(f"FAIL {s}: max {maxd} > {THRESHOLDS['max_max']}")
		ok = False
	if glitch > THRESHOLDS["glitch_max"]:
		print(f"FAIL {s}: glitch {glitch} > {THRESHOLDS['glitch_max']}")
		ok = False
	if not block_ok:
		print(f"FAIL {s}: color blocks differ >30 (likely 422/444 subsampling swap)")
		ok = False
	if not checker_ok:
		print(f"FAIL {s}: checker differ >40 (likely 444 full-res mixup)")
		ok = False

	if ok:
		print(f"OK {s}: mean {mean:.2f} max {maxd} glitch {glitch} blocks {block_ok} checker {checker_ok}")
	return ok

ok_all = True
for s in ["420", "422", "444"]:
	if not check_one(s):
		ok_all = False

if not ok_all:
	print("\nverify: FAILED — possible 2x1 vertical subsampling regression (see backend/zune-jpeg/src/mcu.rs:235)")
	sys.exit(1)
else:
	print("\nverify: PASSED (420/422/444 within thresholds, scalar reference untouched)")

PY

echo "Cleaning up /tmp/verify.*.png..."
rm -f /tmp/verify.ref.*.png /tmp/verify.decoded.*.png
