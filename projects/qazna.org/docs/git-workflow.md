# Local Git workflow

The active workspace is connected to `https://github.com/DauletBai/tartarus.git` through local development branches. Check `git branch --show-current` before publishing; the command below publishes that branch under its own name. Qazna lives at `projects/qazna.org/`; Git commands also work from that directory. This replaces the earlier API-only publication workflow with a local Git index, commits and IDE status/diffs.

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
git push -u origin HEAD
```

`update_manifest.py` updates only Qazna/CI entries and preserves unrelated manifest entries. Review the staged paths before committing. Changes to historical migrations are not allowed. Do not merge or force-push without authorization. Keep published commits and the local branch synchronized; verify the remote head and CI result before claiming publication is complete.

IDE decorations show Git working-tree state, not whether a file has ever been edited. New files normally appear as untracked/added and edited tracked files as modified; exact colors depend on the IDE and theme. Once committed, those marks disappear because the working tree is clean. A refresh or reopening the workspace may be needed after the initial Git setup.

## Publication after PR #1

PR #1 was merged on 7 November 2026 at `4e859a5`. The lifecycle stage is
published through [PR #2](https://github.com/DauletBai/tartarus/pull/2) on
`codex/qazna-lifecycle-integration`. The author workspace was reconciled with
published commit `69ca7ef` after backing up every changed file and Git metadata,
comparing all project files with the publication worktree, and verifying that
working-file bytes were unchanged. The local manuscript README remains intact.

PR #2 subsequently passed both GitHub checks and merged as `0ab50cb` on
7 November 2026. The next daily-maintenance branch starts from that verified main.
Continue editing and committing in this author workspace and use the explicit
push command above.
Do not copy stale files back from the older publication worktree. A saved remote
branch is distinct from a merged change on `main`; check the PR and CI before
reporting that the default branch has been updated.

## Main branch protection

The warning that `main` is not protected concerns remote repository settings;
committing or pushing source files does not activate protection. The prepared
[main ruleset](main-protection.ruleset.json) blocks deletion and force pushes,
without adding bypass actors or preventing ordinary fast-forward updates.

To activate it, open the repository Settings, Rules / Rulesets, then New ruleset,
Import a ruleset. Select this JSON, check that the target is `refs/heads/main` and
Enforcement is Active, then click Create. Finally inspect the repository rules
page to confirm activation. Saving this JSON in Git alone does not apply it.

This is a minimal protection policy, not mandatory code review. Qazna CI currently
has path filters and does not run for book-only changes. Do not require its check
globally until the workflow has an always-reporting gate; otherwise unrelated
book PRs can remain blocked waiting for a check that never starts. PR review and
successful CI remain publication practices, but are not enforced by this ruleset.

References: [GitHub ruleset import instructions](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/managing-rulesets-for-a-repository)
and [repository rules API](https://docs.github.com/en/rest/repos/rules).

## Repository overview versus project folder

GitHub's repository About/topics/language overview belongs to the repository main
page, not to this nested project directory. This is not a missing Qazna asset or
README CSS setting. The project README therefore includes a compact information
card linking its scope, architecture, evidence, licences and CI. Repository-wide
metadata still describes the whole Tartarus archive. See
[GitHub's main-page topic documentation](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/classifying-your-repository-with-topics).

PR #3 passed both remote checks and merged as `54a0db6` on 7 November 2026.
The nutrient/order stage starts on `codex/qazna-nutrition-orders` from that main.

PR #4 passed both GitHub checks and merged as `3fd6927`. The seasonal-food stage
starts from that main commit on `codex/qazna-seasonal-food`. Publication and
verification dates for the completed stage are recorded with its PR.
