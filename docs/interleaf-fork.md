# Interleaf's rtex fork

`upstream` is HenryXiaoYang/realtime-tex; `origin` is Zack-Will/realtime-tex.
Every topic starts at `v0.0.3` (`1f68183`). `interleaf` combines them with
`git merge --no-ff`; upstreamable topics have no dependency on fork-only changes.

| Topic branch | Purpose | Upstream status |
|---|---|---|
| `fix/acmart-heading-guard` | Heading guards, two-column capture, float geometry and image resource indices, with TeX regressions | not yet proposed |
| `feat/layout-pass-pdf` | Opt-in `SessionConfig.pass_pdf`, C JSON `pass_pdf`, `serve --pass-pdf`; stable PDF retained through two later layouts | not yet proposed |
| `fix/verify-form-bbox` | Compare included PDF figures using Form XObject BBox and Matrix, including four-corner bounds | not yet proposed |
| `feat/serve-binary-dl` | Opt-in `serve --binary-dl`: standard padded base64 in `dl_b64`, retaining JSON-lines framing | not yet proposed |
| `ci/interleaf-release` | Linux release build and publication after the existing Linux CI suite | not yet proposed (fork-only; no upstream proposal planned) |

The integration branch also carries this document and any merge resolutions.
See [embedding.md](embedding.md) for configuration and protocol details and
[development.md](development.md) for local testing. On a small host, build with
`nice -n 10 cargo build --workspace --release -j 2` and run test suites sequentially
(`cargo test --workspace --release -j 2 -- --test-threads=1`).

After reviewing and testing `interleaf`, the release operator can cut the next tag:

```sh
git switch interleaf
git tag -a interleaf-0.0.3-1 -m 'Interleaf rtex 0.0.3 patchset 1'
git push origin interleaf
git push origin interleaf-0.0.3-1
```

Choose an unused `interleaf-<upstream version>-<n>` tag for each release. The
`interleaf-release.yml` workflow runs only on pushed `interleaf-*` tags. It calls
`ci.yml` with `linux_only: true`, preserving the upstream Linux unit, integration
and performance jobs. It builds on Ubuntu 22.04 with the upstream `dist` profile
and `scripts/package.sh`, smoke-tests the packaged support-file discovery, and
publishes `rtex-x86_64-unknown-linux-gnu.tar.gz` plus its `.tar.gz.sha256` only after
both CI and packaging succeed. Downstream images should pin a specific release
and verify the archive hash. Creating this patchset does not push or tag anything.
