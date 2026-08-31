#!/usr/bin/env bash
# scripts/generate-sample.sh — reproducible synthetic test image for artefact
# Generates assets/sample.png (1600x1200, RGB24) via ffmpeg lavfi.
# Easy for LLMs to read/reason: 4 labeled sections + header/footer.
#
# Layout (1600x1200):
#   Header (0-120):      dark #0f172a, title ARTEFACT + subtitle
#   Subheader (120-162): #e2e8f0, sRGB/4:2:0 hints
#   1. Gradients (200-380):   LUMA black→white, CHROMA red→blue, RAINBOW 6-color  — DCT staircasing
#   2. Color Blocks (440-590): 6 saturated fills (red/green/blue/yellow/magenta/cyan) — chroma bleeding
#   3. Patterns (650-900):    diagonal 8px, thin 1/2/4/6px lines, checker 8×8 — high-freq
#   4. Text (960-1135):       24/18/16/12pt black/red/blue/green + white-on-color boxes — text bleeding
#   Footer (1160-1200):  provenance
#
# Usage:
#   ./scripts/generate-sample.sh [output]        # default: assets/sample.png
#   ./scripts/generate-sample.sh assets/sample.png 1600 1200
#   just generate-sample            # if recipe added to justfile
#
# Requirements: ffmpeg with libfreetype + DejaVu fonts (fonts-dejavu-core)
#   sudo apt-get update && sudo apt-get install -y ffmpeg fonts-dejavu-core

set -euo pipefail

OUT="${1:-assets/sample.png}"
W="${2:-1600}"
H="${3:-1200}"

if [[ "$OUT" == "-h" || "$OUT" == "--help" ]]; then
  cat <<'USAGE'
Usage: ./scripts/generate-sample.sh [output] [width] [height]
  output  path to PNG (default: assets/sample.png)
  width   image width  (default: 1600)
  height  image height (default: 1200)
Example:
  ./scripts/generate-sample.sh
  ./scripts/generate-sample.sh /tmp/sample.png 800 600
USAGE
  exit 0
fi

# --- checks ---
if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "error: ffmpeg not found. Install with: sudo apt-get install ffmpeg" >&2
  exit 1
fi
# check drawtext (avoid SIGPIPE 141 with pipefail: grep -q exits early)
set +o pipefail
if ! ffmpeg -hide_banner -h filter=drawtext 2>&1 | grep -q "fontfile"; then
  echo "error: ffmpeg built without --enable-libfreetype (drawtext missing)" >&2
  exit 1
fi
set -o pipefail

# --- font discovery (fc-match preferred, fallback to known paths) ---
FONT_BOLD="$(fc-match --format "%{file}\n" "DejaVu Sans:style=Bold" 2>/dev/null | head -n1 || true)"
FONT_REG="$(fc-match --format "%{file}\n" "DejaVu Sans:style=Book" 2>/dev/null | head -n1 || true)"
FONT_MONO="$(fc-match --format "%{file}\n" "DejaVu Sans Mono:style=Book" 2>/dev/null | head -n1 || true)"

# fallbacks if fc-match failed or file missing
if [[ -z "${FONT_BOLD:-}" || ! -f "$FONT_BOLD" ]]; then FONT_BOLD="/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"; fi
if [[ -z "${FONT_REG:-}" || ! -f "$FONT_REG" ]]; then FONT_REG="/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"; fi
if [[ -z "${FONT_MONO:-}" || ! -f "$FONT_MONO" ]]; then FONT_MONO="/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"; fi

for f in "$FONT_BOLD" "$FONT_REG" "$FONT_MONO"; do
  if [[ ! -f "$f" ]]; then
    echo "error: font not found: $f (install fonts-dejavu-core)" >&2
    exit 1
  fi
done

mkdir -p "$(dirname "$OUT")"

echo "Generating $OUT (${W}x${H})..."
echo "  fonts: $FONT_REG, $FONT_BOLD, $FONT_MONO"

# Derive derived sizes (kept fixed for 1600x1200; scaled proportionally if W/H differ)
# For custom sizes we keep original coordinates if W==1600 && H==1200, else scale linearly.
# Simplest: require 1600x1200 original design — warn if different.
if [[ "$W" != "1600" || "$H" != "1200" ]]; then
  echo "warn: custom size ${W}x${H} uses same overlay coords as 1600x1200 — may need manual adjust" >&2
fi

ffmpeg -y \
  -f lavfi -i "color=c=white:s=${W}x${H}:r=25,format=rgb24" \
  -f lavfi -i "gradients=s=520x180:c0=black:c1=white:x0=0:y0=0:x1=520:y1=0:seed=0,format=rgb24" \
  -f lavfi -i "gradients=s=520x180:c0=red:c1=blue:x0=0:y0=0:x1=520:y1=0:seed=0,format=rgb24" \
  -f lavfi -i "gradients=s=480x180:c0=red:c1=yellow:c2=lime:c3=cyan:c4=blue:c5=magenta:nb_colors=6:x0=0:y0=0:x1=480:y1=0:seed=0,format=rgb24" \
  -f lavfi -i "color=c=white:s=480x250:r=25,format=rgb24" \
  -f lavfi -i "color=c=white:s=520x250:r=25,format=rgb24" \
  -filter_complex "\
[0:v] \
drawbox=x=0:y=0:w=1600:h=120:color=#0f172a:t=fill, \
drawbox=x=0:y=120:w=1600:h=42:color=#e2e8f0:t=fill, \
drawtext=fontfile=${FONT_BOLD}:text='ARTEFACT  —  Synthetic Test Image':fontcolor=white:fontsize=46:x=(w-text_w)/2:y=26, \
drawtext=fontfile=${FONT_REG}:text='1600x1200  •  Gradients · Color Blocks · Sharp Edges · High-Frequency Patterns  •  JPEG artefact / DCT solver test':fontcolor=white:fontsize=16:x=(w-text_w)/2:y=82, \
drawtext=fontfile=${FONT_MONO}:text='sRGB  •  8-bit  •  White background  •  Designed for 4\\:2\\:0 / 4\\:2\\:2 / 4\\:4\\:4 & deblocking evaluation':fontcolor=#334155:fontsize=12:x=(w-text_w)/2:y=131, \
drawtext=fontfile=${FONT_BOLD}:text='1. GRADIENTS — DCT staircasing & smooth recovery':fontcolor=#0f172a:fontsize=14:x=20:y=175, \
drawtext=fontfile=${FONT_REG}:text='LUMA  black→white':fontcolor=#1e293b:fontsize=11:x=20:y=388, \
drawtext=fontfile=${FONT_REG}:text='CHROMA  red→blue':fontcolor=#1e293b:fontsize=11:x=560:y=388, \
drawtext=fontfile=${FONT_REG}:text='RAINBOW  6-color  red-yellow-lime-cyan-blue-magenta':fontcolor=#1e293b:fontsize=11:x=1100:y=388 \
[base]; \
[base][1:v] overlay=20:200:shortest=1:format=rgb [t1]; \
[t1][2:v] overlay=560:200:shortest=1:format=rgb [t2]; \
[t2][3:v] overlay=1100:200:shortest=1:format=rgb [t3]; \
[4:v] geq=r='255*eq(mod(floor(X/8)+floor(Y/8),2),0)':g='255*eq(mod(floor(X/8)+floor(Y/8),2),0)':b='255*eq(mod(floor(X/8)+floor(Y/8),2),0)':a='255' [chk]; \
[5:v] geq=r='255*lt(mod(X+Y,16),8)':g='255*lt(mod(X+Y,16),8)':b='255*lt(mod(X+Y,16),8)':a='255' [diag]; \
[t3][chk] overlay=1100:650:shortest=1:format=rgb [t4]; \
[t4][diag] overlay=20:650:shortest=1:format=rgb [t5]; \
[t5] \
drawbox=x=20:y=200:w=520:h=180:color=#cbd5e1:t=2, \
drawbox=x=560:y=200:w=520:h=180:color=#cbd5e1:t=2, \
drawbox=x=1100:y=200:w=480:h=180:color=#cbd5e1:t=2, \
drawtext=fontfile=${FONT_BOLD}:text='2. COLOR BLOCKS — Chroma subsampling (bleeding at sharp edges)':fontcolor=#0f172a:fontsize=14:x=20:y=415, \
drawbox=x=20:y=440:w=251:h=150:color=#ff0000:t=fill, \
drawbox=x=281:y=440:w=251:h=150:color=#00ff00:t=fill, \
drawbox=x=542:y=440:w=251:h=150:color=#0000ff:t=fill, \
drawbox=x=803:y=440:w=251:h=150:color=#ffff00:t=fill, \
drawbox=x=1064:y=440:w=251:h=150:color=#ff00ff:t=fill, \
drawbox=x=1325:y=440:w=251:h=150:color=#00ffff:t=fill, \
drawbox=x=20:y=440:w=251:h=150:color=#1e293b:t=2, \
drawbox=x=281:y=440:w=251:h=150:color=#1e293b:t=2, \
drawbox=x=542:y=440:w=251:h=150:color=#1e293b:t=2, \
drawbox=x=803:y=440:w=251:h=150:color=#1e293b:t=2, \
drawbox=x=1064:y=440:w=251:h=150:color=#1e293b:t=2, \
drawbox=x=1325:y=440:w=251:h=150:color=#1e293b:t=2, \
drawtext=fontfile=${FONT_MONO}:text='RED #FF0000':fontcolor=white:fontsize=11:x=70:y=505, \
drawtext=fontfile=${FONT_MONO}:text='GREEN #00FF00':fontcolor=black:fontsize=11:x=315:y=505, \
drawtext=fontfile=${FONT_MONO}:text='BLUE #0000FF':fontcolor=white:fontsize=11:x=582:y=505, \
drawtext=fontfile=${FONT_MONO}:text='YELLOW #FFFF00':fontcolor=black:fontsize=11:x=830:y=505, \
drawtext=fontfile=${FONT_MONO}:text='MAGENTA #FF00FF':fontcolor=white:fontsize=11:x=1095:y=505, \
drawtext=fontfile=${FONT_MONO}:text='CYAN #00FFFF':fontcolor=black:fontsize=11:x=1355:y=505, \
drawtext=fontfile=${FONT_REG}:text='Adjacent saturated colors create hard chroma edges — ideal to compare 4\\:4\\:4 vs 4\\:2\\:0':fontcolor=#475569:fontsize=11:x=20:y=600, \
drawtext=fontfile=${FONT_BOLD}:text='3. PATTERNS — High frequency & thin structures':fontcolor=#0f172a:fontsize=14:x=20:y=625, \
drawbox=x=20:y=650:w=520:h=250:color=#1e293b:t=2, \
drawbox=x=560:y=650:w=520:h=250:color=#1e293b:t=2, \
drawbox=x=1100:y=650:w=480:h=250:color=#1e293b:t=2, \
drawtext=fontfile=${FONT_REG}:text='DIAGONAL STRIPES  8px  (X+Y mod 16)':fontcolor=#1e293b:fontsize=11:x=20:y=910, \
drawtext=fontfile=${FONT_REG}:text='THIN LINES  1px / 2px / 4px  —  horizontal + vertical':fontcolor=#1e293b:fontsize=11:x=560:y=910, \
drawtext=fontfile=${FONT_REG}:text='CHECKER  8x8  (1\\:1)':fontcolor=#1e293b:fontsize=11:x=1100:y=910, \
drawbox=x=570:y=670:w=500:h=2:color=black:t=fill, \
drawbox=x=570:y=690:w=500:h=4:color=black:t=fill, \
drawbox=x=570:y=720:w=500:h=1:color=black:t=fill, \
drawbox=x=570:y=740:w=500:h=6:color=black:t=fill, \
drawbox=x=570:y=770:w=500:h=1:color=red:t=fill, \
drawbox=x=570:y=790:w=500:h=2:color=blue:t=fill, \
drawbox=x=570:y=815:w=500:h=1:color=black:t=fill, \
drawbox=x=580:y=660:w=2:h=230:color=black:t=fill, \
drawbox=x=600:y=660:w=4:h=230:color=black:t=fill, \
drawbox=x=630:y=660:w=1:h=230:color=black:t=fill, \
drawbox=x=650:y=660:w=6:h=230:color=black:t=fill, \
drawbox=x=680:y=660:w=1:h=230:color=red:t=fill, \
drawbox=x=700:y=660:w=2:h=230:color=blue:t=fill, \
drawbox=x=730:y=660:w=1:h=230:color=black:t=fill, \
drawtext=fontfile=${FONT_MONO}:text='thin':fontcolor=#64748b:fontsize=9:x=1075:y=675, \
drawtext=fontfile=${FONT_MONO}:text='thick':fontcolor=#64748b:fontsize=9:x=1075:y=695, \
drawtext=fontfile=${FONT_BOLD}:text='4. TEXT SHARPNESS — Colored text on white / white on color (chroma bleeding)':fontcolor=#0f172a:fontsize=14:x=20:y=935, \
drawbox=x=20:y=960:w=1560:h=175:color=#f1f5f9:t=fill, \
drawbox=x=20:y=960:w=1560:h=175:color=#cbd5e1:t=2, \
drawtext=fontfile=${FONT_REG}:text='The quick brown fox jumps over the lazy dog  —  24pt  BLACK':fontcolor=black:fontsize=24:x=40:y=975, \
drawtext=fontfile=${FONT_REG}:text='The quick brown fox jumps over the lazy dog  —  18pt  RED  (#FF0000)':fontcolor=#ff0000:fontsize=18:x=40:y=1010, \
drawtext=fontfile=${FONT_REG}:text='The quick brown fox jumps over the lazy dog  —  18pt  BLUE  (#0000FF)':fontcolor=#0000ff:fontsize=18:x=40:y=1035, \
drawtext=fontfile=${FONT_REG}:text='The quick brown fox jumps over the lazy dog  —  16pt  GREEN  (#00AA00) — small text bleeds most with 4\\:2\\:0':fontcolor=#00aa00:fontsize=16:x=40:y=1065, \
drawtext=fontfile=${FONT_MONO}:text='0123456789  ABCDEFG  —  12pt mono — hard to keep sharp after JPEG q=20-60':fontcolor=black:fontsize=12:x=40:y=1092, \
drawbox=x=1200:y=985:w=350:h=33:color=#ff0000:t=fill, \
drawtext=fontfile=${FONT_BOLD}:text='WHITE on RED':fontcolor=white:fontsize=15:x=1265:y=993, \
drawbox=x=1200:y=1025:w=350:h=33:color=#0000ff:t=fill, \
drawtext=fontfile=${FONT_BOLD}:text='WHITE on BLUE':fontcolor=white:fontsize=15:x=1258:y=1033, \
drawbox=x=1200:y=1065:w=350:h=33:color=#ffcc00:t=fill, \
drawtext=fontfile=${FONT_BOLD}:text='BLACK on YELLOW':fontcolor=black:fontsize=15:x=1255:y=1073, \
drawbox=x=0:y=1160:w=1600:h=40:color=#0f172a:t=fill, \
drawtext=fontfile=${FONT_MONO}:text='ffmpeg lavfi  •  1600x1200  •  PNG  •  artefact sample  •  gradients / colors / patterns / text — compare JPG (q 30-80) vs decoded PNG':fontcolor=white:fontsize=10:x=(w-text_w)/2:y=1173, \
format=rgb24 \
[final] \
" -map "[final]" -frames:v 1 -pix_fmt rgb24 -y "$OUT"

echo "Done: $OUT"
ffprobe -hide_banner -i "$OUT" 2>&1 | grep -E "Stream|Video" || true
ls -lh "$OUT"
