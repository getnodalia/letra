# Letra upstream

| Field | Value |
|---|---|
| Upstream | `storytold/wordcraft` |
| Last observed | `69ea44776d8e01fa96ad1c4766c2675896f4df01` |
| Last evaluated | `8548b33be1c9238e6d42fb2a72fe27ccdd56be7f` |
| Last integrated | `22a73d9f2d10de6ee52780ab4a6019b864d9c598` (278 commits since the last sync) on 2026-11-18 |
| Skipped | the community button and the brand files (Letra has neither) |
| Adapted | every name (`scripts/rebrand.py`), the app icon, the Nodalia interface |
| Conflicting | `README.md` (kept ours), `AGENTS.md` (upstream plus our quality contract) |

## How the sync works

Upstream's code arrives renamed: `scripts/rebrand.py` applies `.github/rebrand.toml` (WordCraft
becomes Letra, internal names included; links to the shared libraries point at
getnodalia/foundation) to upstream's tree, and the weekly sync
(`.github/workflows/upstream-sync.yml`) records the result as one mirror commit: its only parent is
the previous mirror commit, its subject is `Upstream <sha> with this app's names`, and an
`Upstream-Commit:` trailer names the upstream commit it renames. The sync merges that commit into a
branch and opens a pull request; files listed in `.github/upstream-keep` keep this repository's
version. No mirror branch is pushed, so merge sync pull requests with a merge commit: a squash would
drop the mirror commit the next sync starts from.

The mirror starts at the renamed `8548b33b`, merged into `main` (with this repository's tree) by the
pull request that renamed the app. `python3 scripts/rebrand.py check` fails when an old name comes
back; a line that must keep one says `rebrand: keep`.