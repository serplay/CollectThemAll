#!/usr/bin/env bash
#
# Cut a release by bumping the version and pushing a tag.
#
# GitHub Actions (.github/workflows/release.yml) then builds the installers for
# Windows, macOS (Apple Silicon + Intel) and Linux on cloud runners, and
# publishes a NON-DRAFT GitHub Release marked as "latest" with the installers
# attached. This script does NOT build or publish locally — it just cuts the tag.
#
# Usage:
#   scripts/release.sh <version> [options]
#
#   <version>          e.g. 1.1.0   (no leading "v"; the tag becomes v1.1.0)
#
# Options:
#   --push-branch      also push the current branch (default: push only the tag)
#   --watch            after pushing, stream the GitHub Actions run in the terminal
#   -h | --help        show this help
#
# Requires: git, and (for --watch / preflight) an authenticated `gh`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

VERSION=""
PUSH_BRANCH=0
WATCH=0

usage() { sed -n '2,24p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }
die()   { echo "ERROR: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --push-branch) PUSH_BRANCH=1; shift ;;
    --watch)       WATCH=1; shift ;;
    -h|--help)     usage 0 ;;
    -*)            echo "Unknown option: $1" >&2; usage 1 ;;
    *)             if [ -z "$VERSION" ]; then VERSION="$1"; else echo "Unexpected arg: $1" >&2; usage 1; fi; shift ;;
  esac
done

[ -n "$VERSION" ] || { echo "A version is required." >&2; usage 1; }
VERSION="${VERSION#v}"
echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+([-+.][0-9A-Za-z.-]+)?$' \
  || die "Version '$VERSION' is not valid semver (expected e.g. 1.2.3)."
TAG="v$VERSION"

# ---- preflight ---------------------------------------------------------------
[ -z "$(git status --porcelain)" ] || die "Working tree is dirty. Commit/stash first."
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && die "Tag $TAG already exists locally."
if git ls-remote --exit-code --tags origin "$TAG" >/dev/null 2>&1; then
  die "Tag $TAG already exists on origin."
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
echo ">> Cutting $TAG from branch '$BRANCH' (CI will build + publish)"

# ---- 1. bump version in all manifests + the lockfile ------------------------
echo ">> Bumping version to $VERSION"
perl -0pi -e 's/("version"\s*:\s*")[^"]+(")/${1}'"$VERSION"'${2}/' package.json
perl -0pi -e 's/("version"\s*:\s*")[^"]+(")/${1}'"$VERSION"'${2}/' src-tauri/tauri.conf.json
# Cargo.toml: version inside the [package] section only (anchored to file start
# or a newline, so dependency version lines are never touched).
perl -0pi -e 's/((?:\A|\n)\[package\][^\[]*?\nversion\s*=\s*")[^"]+(")/${1}'"$VERSION"'${2}/s' src-tauri/Cargo.toml
# Cargo.lock: the entry for this crate (so the committed lockfile stays in sync).
perl -0pi -e 's/(name = "cta-collectthemall"\nversion = ")[^"]+(")/${1}'"$VERSION"'${2}/' src-tauri/Cargo.lock

grep -q "\"version\": \"$VERSION\"" package.json              || die "Failed to bump package.json"
grep -q "\"version\": \"$VERSION\"" src-tauri/tauri.conf.json || die "Failed to bump tauri.conf.json"
grep -q "^version = \"$VERSION\""    src-tauri/Cargo.toml      || die "Failed to bump Cargo.toml"

# ---- 2. commit, tag, push ----------------------------------------------------
echo ">> Committing version bump"
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -q -m "chore(release): $TAG"

git tag -a "$TAG" -m "$TAG"
echo ">> Pushing tag $TAG (this triggers the Release workflow)"
git push origin "refs/tags/$TAG"
if [ "$PUSH_BRANCH" -eq 1 ]; then
  echo ">> Pushing branch $BRANCH"
  git push origin "refs/heads/$BRANCH:refs/heads/$BRANCH"
fi

# ---- 3. point the user at the run -------------------------------------------
REPO_URL="$(git remote get-url origin | sed -E 's#git@github.com:#https://github.com/#; s#\.git$##')"
echo ""
echo ">> Tag pushed. GitHub Actions is now building + publishing the release."
echo "   Actions:  ${REPO_URL}/actions/workflows/release.yml"
echo "   Release:  ${REPO_URL}/releases/tag/${TAG}  (appears once CI finishes)"

if [ "$WATCH" -eq 1 ]; then
  if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
    echo ">> Watching the latest run..."
    sleep 5
    gh run watch "$(gh run list --workflow=release.yml --limit 1 --json databaseId --jq '.[0].databaseId')" --exit-status || true
  else
    echo "   (gh not authenticated — skipping --watch)"
  fi
fi
