#!/usr/bin/env bash
# Usage (repository root): bash tex/tests/images.sh /absolute/scratch/directory
set -euo pipefail
scratch=${1:?provide a scratch directory}
mkdir -p "$scratch/project"
cp fixtures/corpus/layout/figure.png "$scratch/project/a.png"
cp fixtures/corpus/layout/figure.png "$scratch/project/b.png"
cat > "$scratch/project/main.tex" <<'TEX'
\documentclass{article}
\usepackage{graphicx}
\begin{document}
First resource: \includegraphics[width=1cm]{a.png}.

Second resource: \includegraphics[width=2cm]{b.png}.

Reused first resource: \includegraphics[width=3cm]{a.png}.
\end{document}
TEX
export RTEX_TEXDIR="$PWD/tex"
rtex verify --project "$scratch/project" --build "$scratch/verify" --min-eligible 3
texlua tex/tests/images.lua "$scratch/verify/capture"
