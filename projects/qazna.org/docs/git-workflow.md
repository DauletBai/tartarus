# Local Git workflow

The active workspace is connected to `https://github.com/DauletBai/tartarus.git` on `codex/qazna-rust-provisioning`, tracking the same origin branch. Qazna lives at `projects/qazna.org/`; Git commands also work from that directory. This replaces the earlier API-only publication workflow with a local Git index, commits and IDE status/diffs.

## Scope of this workspace

The author workspace also contains local manuscript and research material that differs from the public archive. Its non-cone sparse checkout includes:

```text
/projects/qazna.org/
/.github/workflows/qazna.yml
/MANIFEST.csv
```

Local Git metadata enables `sparse.expectFilesOutsideOfPatterns` and marks tracked paths outside that selection as skip-worktree. Local excludes hide unrelated untracked material. The author's book README and other local book files are preserved. The separate older `tartarus/` checkout is not moved or rewritten.

The clone uses Git's blob filter to avoid downloading the book/archive payload merely to work on Qazna. Current Qazna baseline blobs are cached for offline diffs. Reading older or excluded blobs can require network access. To work on the complete public book archive, use a separate checkout; keep this workspace's selection focused on Qazna.

## Daily work

From the repository root:

```sh
git status --short
git diff -- projects/qazna.org
cd projects/qazna.org
PG_BIN=/opt/homebrew/opt/postgresql@16/bin python3 scripts/check.py
python3 scripts/update_manifest.py
cd ../..
git add -- projects/qazna.org .github/workflows/qazna.yml MANIFEST.csv
git diff --cached --stat
git commit -m "Describe the verified change"
git push origin codex/qazna-rust-provisioning
```

`update_manifest.py` updates only Qazna/CI entries and preserves unrelated manifest entries. Review the staged paths before committing. Changes to historical migrations are not allowed. Do not merge or force-push without authorization. Keep published commits and the local branch synchronized; verify the remote head and CI result before claiming publication is complete.

IDE decorations show Git working-tree state, not whether a file has ever been edited. New files normally appear as untracked/added and edited tracked files as modified; exact colors depend on the IDE and theme. Once committed, those marks disappear because the working tree is clean. A refresh or reopening the workspace may be needed after the initial Git setup.

## Publication after PR #1

PR #1 was merged on 7 November 2026 at `4e859a5`. The next lifecycle stage is
published from the separate project-scoped worktree on
`codex/qazna-lifecycle-integration`. The original author workspace is deliberately
left on its existing branch with the edited project files preserved; it is not a
clean checkout of this new branch. Do not reset it or replace its manuscript README.

For the next-stage publication, copy only the reviewed project files into the
separate worktree, verify them against the tested source, regenerate its manifest,
and commit/push that development branch. Check its actual remote commit before
claiming synchronization. Reconcile the original workspace separately with a
complete backup and review of its local files.


## Repository overview versus project folder

GitHub's repository About/topics/language overview belongs to the repository main
page, not to this nested project directory. This is not a missing Qazna asset or
README CSS setting. The project README therefore includes a compact information
card linking its scope, architecture, evidence, licences and CI. Repository-wide
metadata still describes the whole Tartarus archive. See
[GitHub's main-page topic documentation](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/classifying-your-repository-with-topics).
