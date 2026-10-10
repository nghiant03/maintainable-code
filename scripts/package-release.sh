#!/usr/bin/env bash
# Package an already-built host executable with dependency license notices.
set -euo pipefail

binary="${1:?Usage: package-release.sh BINARY TARGET OUTPUT_DIRECTORY}"
target="${2:?Target triple required}"
output="${3:?Output directory required}"
root="$(cd "$(dirname "$0")/.." && pwd)"
binary="$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")"
version="$("$binary" --version)"
version="${version#maintainable-code }"
if [[ -n "${RELEASE_TAG:-}" && "$RELEASE_TAG" != "v$version" ]]; then
    echo "Release tag $RELEASE_TAG does not match package version $version" >&2
    exit 1
fi
mkdir -p "$output"
output="$(cd "$output" && pwd)"
name="maintainable-code-v$version-$target"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/$name/licenses"
cp "$binary" "$stage/$name/$(basename "$binary")"
cp "$root/README.md" "$stage/$name/README.md"
cp "$root/docs/installation.md" "$stage/$name/INSTALLATION.md"
for license in "$root/"LICENSE "$root/"LICENSE.md; do
    if [[ -f "$license" ]]; then
        cp "$license" "$stage/$name/"
    fi
done
cargo metadata --locked --format-version 1 --filter-platform "$target" --manifest-path "$root/Cargo.toml" > "$stage/all-metadata.json"
jq '[.resolve.nodes[].id] as $ids | .packages |= map(select(.id as $id | $ids | index($id)))' "$stage/all-metadata.json" > "$stage/metadata.json"
jq '[.packages[] | {name,version,license,repository}]' "$stage/metadata.json" > "$stage/$name/THIRD_PARTY.json"
while IFS=$'\t' read -r crate version manifest; do
    if command -v cygpath >/dev/null; then
        manifest="$(cygpath -u "$manifest")"
    fi
    directory="$(dirname "$manifest")"
    notices="$stage/$name/licenses/$crate-$version"
    mkdir -p "$notices"
    count=0
    for file in "$directory"/LICENSE* "$directory"/LICENCE* "$directory"/COPYING* "$directory"/NOTICE* "$directory"/license*; do
        if [[ -f "$file" ]]; then
            cp "$file" "$notices/"
            count=$((count + 1))
        fi
    done
    if [[ "$count" == 0 ]]; then
        supplement="$root/third-party/tree-sitter-notices.md"
        if grep -Fq "| $crate | $version |" "$supplement"; then
            cp "$supplement" "$notices/UPSTREAM-LICENSES.md"
        else
            echo "No license notices found for $crate $version; review before release" >&2
            exit 1
        fi
    fi
done < <(jq -r '.packages[] | select(.name != "maintainable-code") | [.name,.version,.manifest_path] | @tsv' "$stage/metadata.json")
tar -czf "$output/$name.tar.gz" -C "$stage" "$name"
(
    cd "$output"
    if command -v sha256sum >/dev/null; then
        sha256sum "$name.tar.gz" > "$name.tar.gz.sha256"
    else
        shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256"
    fi
)
echo "$output/$name.tar.gz"
