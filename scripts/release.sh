#!/usr/bin/env bash
#
# Cut a release: bump version -> build -> tag -> push tag -> publish a
# (non-draft, marked-latest) GitHub release with the installers attached.
#
# Usage:
#   scripts/release.sh <version> [options]
#
#   <version>            e.g. 1.1.0   (no leading "v"; the tag becomes v1.1.0)
#
# Options:
#   --notes "<text>"     release notes body (default: auto-generated from commits)
#   --push-branch        also push the current branch (default: push only the tag)
#   --skip-build         reuse existing artifacts in target/release/bundle
#   --prerelease         mark the GitHub release as a pre-release
#   -h | --help          show this help
#
# Requires: node/npm, cargo, the Tauri CLI, and an authenticated `gh`.

set -euo pipefail

# ---- locate repo root (script lives in <repo>/scripts) -----------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# ---- parse args --------------------------------------------------------------
VERSION=""
NOTES=""
PUSH_BRANCH=0
SKIP_BUILD=0
PRERELEASE=0

usage() { sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

while [ $# -gt 0 ]; do
  case "$1" in
    --notes)       NOTES="${2:-}"; shift 2 ;;
    --push-branch) PUSH_BRANCH=1; shift ;;
    --skip-build)  SKIP_BUILD=1; shift ;;
    --prerelease)  PRERELEASE=1; shift ;;
    -h|--help)     usage 0 ;;
    -*)            echo "Unknown option: $1" >&2; usage 1 ;;
    *)             if [ -z "$VERSION" ]; then VERSION="$1"; else echo "Unexpected arg: $1" >&2; usage 1; fi; shift ;;
  esac
done

die() { echo "ERROR: $*" >&2; exit 1; }

[ -n "$VERSION" ] || { echo "A version is required." >&2; usage 1; }
# Strip an accidental leading "v", then validate as semver-ish (X.Y.Z optionally -tag).
VERSION="${VERSION#v}"
echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+([-+.][0-9A-Za-z.-]+)?$' \
  || die "Version '$VERSION' is not valid semver (expected e.g. 1.2.3)."
TAG="v$VERSION"

# ---- preflight ---------------------------------------------------------------
command -v gh    >/dev/null || die "GitHub CLI 'gh' not found."
gh auth status   >/dev/null 2>&1 || die "gh is not authenticated. Run: gh auth login"
command -v cargo >/dev/null || die "cargo not found."

# Clean working tree (so the version bump is the only change we commit).
[ -z "$(git status --porcelain)" ] || die "Working tree is dirty. Commit/stash first."

# Tag must be new, locally and on the remote.
git rev-parse -q --verify "refs/tags/$TAG" >/dev/null && die "Tag $TAG already exists locally."
if git ls-remote --exit-code --tags origin "$TAG" >/dev/null 2>&1; then
  die "Tag $TAG already exists on origin."
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
echo ">> Releasing $TAG from branch '$BRANCH'"

# ---- 1. bump version in all three manifests ---------------------------------
echo ">> Bumping version to $VERSION"
# package.json: first "version": "..."
perl -0pi -e 's/("version"\s*:\s*")[^"]+(")/${1}'"$VERSION"'${2}/' package.json
# tauri.conf.json: first "version": "..."
perl -0pi -e 's/("version"\s*:\s*")[^"]+(")/${1}'"$VERSION"'${2}/' src-tauri/tauri.conf.json
# Cargo.toml: the version line inside the [package] section only (anchored to
# start-of-file or a newline, so [package] at the very top still matches, and
# dependency version lines elsewhere are left untouched).
perl -0pi -e 's/((?:\A|\n)\[package\][^\[]*?\nversion\s*=\s*")[^"]+(")/${1}'"$VERSION"'${2}/s' src-tauri/Cargo.toml

# Sanity-check the bumps actually took.
grep -q "\"version\": \"$VERSION\"" package.json            || die "Failed to bump package.json"
grep -q "\"version\": \"$VERSION\"" src-tauri/tauri.conf.json || die "Failed to bump tauri.conf.json"
grep -q "^version = \"$VERSION\""    src-tauri/Cargo.toml      || die "Failed to bump Cargo.toml"

# ---- 2. build the release bundles -------------------------------------------
if [ "$SKIP_BUILD" -eq 1 ]; then
  echo ">> Skipping build (--skip-build)"
else
  echo ">> Building release bundles (this can take a while)..."
  npm run tauri build
fi

# Collect the produced installers (Windows .msi/.exe, mac .dmg/.app.tar.gz, linux .deb/.AppImage/.rpm).
BUNDLE_DIR="src-tauri/target/release/bundle"
[ -d "$BUNDLE_DIR" ] || die "Bundle dir not found: $BUNDLE_DIR (did the build run?)"
mapfile -t ARTIFACTS < <(find "$BUNDLE_DIR" -type f \
  \( -iname '*.msi' -o -iname '*.exe' -o -iname '*.dmg' -o -iname '*.AppImage' \
     -o -iname '*.deb' -o -iname '*.rpm' -o -iname '*.app.tar.gz' -o -iname '*.sig' \) | sort)
[ "${#ARTIFACTS[@]}" -gt 0 ] || die "No installer artifacts found under $BUNDLE_DIR."
echo ">> Artifacts to attach:"; printf '   %s\n' "${ARTIFACTS[@]}"

# ---- 3. commit the bump (now includes any Cargo.lock update from the build) --
echo ">> Committing version bump"
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -q -m "chore(release): $TAG"

# ---- 4. tag and push ---------------------------------------------------------
git tag -a "$TAG" -m "$TAG"
echo ">> Pushing tag $TAG"
git push origin "refs/tags/$TAG"
if [ "$PUSH_BRANCH" -eq 1 ]; then
  echo ">> Pushing branch $BRANCH"
  git push origin "refs/heads/$BRANCH:refs/heads/$BRANCH"
fi

# ---- 5. publish the GitHub release (NOT a draft) ----------------------------
echo ">> Creating GitHub release $TAG"
# Note: the tag is already pushed above, so gh attaches the release to it.
# (No --target needed; that only applies when gh itself creates the tag.)
GH_ARGS=( "$TAG" "${ARTIFACTS[@]}" --title "$TAG" )
if [ "$PRERELEASE" -eq 1 ]; then GH_ARGS+=( --prerelease ); else GH_ARGS+=( --latest ); fi
if [ -n "$NOTES" ]; then GH_ARGS+=( --notes "$NOTES" ); else GH_ARGS+=( --generate-notes ); fi

gh release create "${GH_ARGS[@]}"

echo ""
echo ">> Done. Release $TAG is published as the current (non-draft) release."
gh release view "$TAG" --json url,isDraft,isLatest --jq '"   url:      \(.url)\n   isDraft:  \(.isDraft)\n   isLatest: \(.isLatest)"'
