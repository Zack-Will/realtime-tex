#!/usr/bin/env bash
# Usage (repository root): bash tex/tests/floats.sh /absolute/scratch/directory
set -euo pipefail
scratch=${1:?provide a scratch directory}
mkdir -p "$scratch/project"
cp fixtures/corpus/layout/figure.png "$scratch/project/figure.png"
cat > "$scratch/project/main.tex" <<'TEX'
\documentclass[twocolumn]{article}
\usepackage{graphicx}
\begin{document}
A paragraph in the first column.

\begin{figure}[t]
\centering\includegraphics[width=0.8\columnwidth]{figure.png}
\caption{A figure using the column width.}
\end{figure}

\begin{figure*}[t]
\centering\includegraphics[width=0.8\textwidth]{figure.png}
\caption{A figure using the full text width.}
\end{figure*}

\begin{table}[t]
\centering\begin{tabular}{ll}A & B\\ C & D\end{tabular}
\caption{A table in one column.}
\end{table}
A final paragraph in the document.
\end{document}
TEX
export RTEX_TEXDIR="$PWD/tex"
rtex verify --project "$scratch/project" --build "$scratch/verify" --min-eligible 5
echo 'all two-column float tests passed'
