#!/usr/bin/env bash
# Run from the repository root; uses an installed/prebuilt rtex, no cargo.
# Usage: bash tex/tests/acmart.sh /absolute/scratch/directory
set -euo pipefail
root=$PWD
scratch=${1:?provide a scratch directory}
mkdir -p "$scratch"
export RTEX_TEXDIR="$root/tex"
export TEXINPUTS="$root/tex/latex//:${TEXINPUTS:-}"
export LUAINPUTS="$root/tex//:${LUAINPUTS:-}"
rtex verify --project "$root/fixtures/corpus/acmart" --build "$scratch/verify" --min-eligible 16
log="$scratch/verify/capture/main.log"
if grep -q 'Class acmart Error' "$log"; then echo 'FAIL acmart class errors'; exit 1; fi
echo 'ok   acmart capture: zero class errors; verify passed'
# A user redefinition must still be diagnosed, both before and after hook installation.
for stage in preamble body; do
  mkdir -p "$scratch/$stage"
  if [ "$stage" = preamble ]; then
    sed 's/\\begin{document}/\\renewcommand\\section[1]{#1}\\begin{document}/' fixtures/corpus/acmart/main.tex > "$scratch/$stage/main.tex"
  else
    sed 's/\\end{document}/\\renewcommand\\section[1]{#1}\\end{document}/' fixtures/corpus/acmart/main.tex > "$scratch/$stage/main.tex"
  fi
  (
    cd "$scratch/$stage"
    export TEXMFCACHE=./tex-cache
    mkdir -p tex-cache
    lualatex -interaction=nonstopmode --jobname=main '\RequirePackage{rtex-capture}\input{main.tex}' > console.txt 2>&1 || true
    grep -q 'Class acmart Error: An attempt to redefine' main.log
  )
  echo "ok   acmart $stage redefinition is still detected"
done
echo 'all acmart tests passed'
