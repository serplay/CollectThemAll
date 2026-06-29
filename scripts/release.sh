#!/usr/bin/env bash
#
# Cut a release: bump the version, push a tag, create the GitHub Release, and let
# GitHub Actions build + attach the cross-platform installers.
#
# Why the script creates the release (and CI only uploads to it):
#   GitHub's Actions GITHUB_TOKEN can UPLOAD assets to an existing release but is
#   not allowed to CREATE one (it 403s with "Resource not accessible by
#   integration"). Your local `gh` uses a PAT that can create releases, so we
#   create the (published) release here; the Release workflow then builds each
#   platform and uploads the installers into it.
#
# Usage:
#   scripts/release.sh <version> [options]
#
#   <version>          e.g. 1.1.2   (no leading "v"; the tag becomes v1.1.2)
#
# Options:
#   --notes "<text>"   release notes body (default: a short generic line)
#   --prerelease       mark the GitHub release as a pre-release (default: latest)
#   --push-branch      also push the current branch (default: push only the tag)
#   --watch            after pushing, stream the GitHub Actions run in the terminal
#   -h | --help        show this help
#
# Requires: git and an authenticated `gh`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

VERSION=""
NOTES="See the assets below to download and install this version."
PRERELEASE=0
PUSH_BRANCH=0
WATCH=0

usage() { sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }
die()   { echo "ERROR: $*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --notes)       NOTES="${2:-}"; shift 2 ;;
    --prerelease)  PRERELEASE=1; shift ;;
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
command -v gh >/dev/null || die "GitHub CLI 'gh' not found."
gh auth status >/dev/null 2>&1 || die "gh is not authenticated. Run: gh auth login"
[ -z "$(git status --porcelain)" ] || die "Working tree is dirty. Commit/stash first."
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && die "Tag $TAG already exists locally."
if git ls-remote --exit-code --tags origin "$TAG" >/dev/null 2>&1; then
  die "Tag $TAG already exists on origin."
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
echo ">> Cutting $TAG from branch '$BRANCH'"

# ---- 1. bump version in all manifests + the lockfile ------------------------
echo ">> Bumping version to $VERSION"
perl -0pi -e 's/("version"\s*:\s*")[^"]+(")/${1}'"$VERSION"'${2}/' package.json
perl -0pi -e 's/("version"\s*:\s*")[^"]+(")/${1}'"$VERSION"'${2}/' src-tauri/tauri.conf.json
# Cargo.toml: version inside the [package] section only (anchored to file start
# or a newline, so dependency version lines are never touched).
perl -0pi -e 's/((?:\A|\n)\[package\][^\[]*?\nversion\s*=\s*")[^"]+(")/${1}'"$VERSION"'${2}/s' src-tauri/Cargo.toml
# Cargo.lock: the entry for this crate (keeps the committed lockfile in sync).
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

# ---- 3. create the (published) GitHub release so CI can upload into it -------
# The build takes minutes, so creating the release now easily wins the race
# against the workflow's upload step. If it somehow already exists, skip.
if gh release view "$TAG" >/dev/null 2>&1; then
  echo ">> Release $TAG already exists — leaving it as-is."
else
  echo ">> Creating GitHub release $TAG"
  REL=( "$TAG" --title "CollectThemAll $TAG" --notes "$NOTES" )
  if [ "$PRERELEASE" -eq 1 ]; then REL+=( --prerelease ); else REL+=( --latest ); fi
  gh release create "${REL[@]}"
fi

# ---- 4. point the user at the run -------------------------------------------
REPO_URL="$(git remote get-url origin | sed -E 's#git@github.com:#https://github.com/#; s#\.git$##')"
echo ""
echo ">> Release created. GitHub Actions is now building + attaching installers."
echo "   Actions:  ${REPO_URL}/actions/workflows/release.yml"
echo "   Release:  ${REPO_URL}/releases/tag/${TAG}"

if [ "$WATCH" -eq 1 ]; then
  echo ">> Watching the latest run..."
  sleep 6
  RUN_ID="$(gh run list --workflow=release.yml --limit 1 --json databaseId --jq '.[0].databaseId')"
  gh run watch "$RUN_ID" --exit-status || true
fi
