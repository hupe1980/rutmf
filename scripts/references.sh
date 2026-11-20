#!/usr/bin/env bash
# The TM Forum OpenAPI documents the coverage and server-conformance suites read.
#
#   scripts/references.sh           fetch each document at its pinned commit and verify it
#   scripts/references.sh --drift   report documents whose upstream `main` has moved on
#
# The documents are not committed: they are fetched into $RUTMF_REFERENCE/oas, by default
# the git-ignored reference directory of this checkout. Each one is pinned to a commit of
# its tmforum-apis repository and to a SHA-256, so the suites check the same bytes on every
# machine until a pin is changed here on purpose.
#
# Adding an API: add a line below (repository, commit, file, sha256 — `shasum -a 256`).
set -euo pipefail
cd "$(dirname "$0")/.."

base=https://raw.githubusercontent.com/tmforum-apis
dest="${RUTMF_REFERENCE:-concepts/reference}/oas"

pins() {
    cat <<'EOF'
TMF620_ProductCatalog 24e69f3026f2b514baa27d7206540fc7b55bcb45 TMF620-Product_Catalog_Management-v5.0.0.oas.yaml a84bc2e456b611c5f2fe3cb82a7f5d6b68e9589a43bcdbe8b5c0a0c3a7429351
TMF621_TroubleTicket cd24d2357e28501b4ab0def951150639443c0058 TMF621-Trouble_Ticket-v5.0.1.oas.yaml dd3d242c643d132833c9a883c562bb56401ad3ef9e79c942a1f987e950be5017
TMF622_ProductOrder a3825f95248eb04fb1c7e2321305bc99631168a4 TMF622-ProductOrdering-v5.0.0.oas.yaml 833d15f23c6e79c43594e68a236821669f1257426dedf51111e98fccdf30a3ba
TMF629_CustomerManagement f60fa840989b88d27fdd0a397c7923f19c9d8168 TMF629-Customer_Management-v5.0.1.oas.yaml fb55f75e317af1ed6e51e484d126e57399e6e1587ad655eda0d47a7f9365a737
TMF632_PartyManagement 11c6a0ecf2b9a7007f43e86601b37e45d9f39b1e TMF632-Party_Management-v5.0.0.oas.yaml 629e0236773c10ddade53598bd3d3c254f0e097e6e5c178aa51eeb8b297aecc2
TMF634_ResourceCatalog 9711751fe23dcb2c6e91bb427f9c6b1e3eb1ddeb TMF634-Resource_Catalog_Management-v5.0.0.oas.yaml 1816898b7a26f2dd97b2f1eadd3598a4e25de9320a662d671b4f58380049c89e
TMF635_UsageManagement 82ea3c6415351b1725b4fd94bcd80ca08425425f TMF635-Product_Usage_Management-v5.0.0.oas.yaml 8fb4f765dc66ba5383a6ce9ea240e54e521a68c3485b53199929c2db98ae58be
TMF637_ProductInventory f598b4a02186071f2794b8f9b4f6904ef0a8aa52 TMF637-ProductInventory-v5.0.0.oas.yaml e3904d0788266b7005d63fce826e0cd07551d7611fc11e15ed42cd414d27c977
TMF638_ServiceInventory 50fc82942e423c81896362d1ab9847f7c918cf7f TMF638-Service_Inventory_Management-v5.0.0.oas.yaml faec2d4b3e551db65cf4a4c1a803534e876f28649f2a2d92ed8297968070a551
TMF639_ResourceInventory 2693a854940de3c6e15b1aade24f981d94b75d3a TMF639-Resource_Inventory_Management-v5.0.0.oas.yaml 60f1bd928326d8cf50bbfe6f6fada952005588c2598f63d72ae8c07d2048b881
TMF642_AlarmManagement 1dcbfa183c4674678853ba02ecd3178ba534e527 TMF642_Alarm_v5.0.1.oas.yaml 821c35c1783a2eedf191556eef983c9cccd3af3c274199efda537da2dafa46ca
TMF666_AccountManagement 8b9c4f62cbf8c41f5b71f5836f48486a3fd856b8 TMF666-Account_Management-v5.0.0.oas.yaml a0cfbd1497fea9648bb7a204e2a32fe6351267e3f7ed9bf99f17cb6ea08c997b
TMF669_PartyRole c55514aa2c28b5f63e328395b747457148800f99 TMF669-Party_Role_Management-v5.0.0.oas.yaml bcd250ef6a3337ef063e572198da1d2c9e7aa1fdd2c0ab8eba87d9fb92dd8645
TMF678_CustomerBill 6d1628e7ac15f077083216f9bccd469e9f1af74f TMF678-CustomerBill-v5.0.0.oas.yaml 661587c5d236e364199218046a6db4266c639ca443b9a4f312ab89f2c8c6c181
TMF679_ProductOfferingQualification 8ab73abb50c5edd3a1ccdd0dc8cd16f0d33032af TMF679-Product_Offering_Qualification-v5.0.0.oas.yaml 244cd3534e414f553857327e0a9cf8d783ee1a9b248048480ed7cb7a27a77485
EOF
}

sha256() {
    if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

if [ "${1:-}" = "--drift" ]; then
    # Informational: TM Forum publishes patch releases without notice, and a pin is moved
    # deliberately — together with the fixtures derived from the document — never silently.
    tmp=$(mktemp); trap 'rm -f "$tmp"' EXIT
    while read -r repo _ file sum; do
        if ! curl -sSfL "$base/$repo/main/$file" -o "$tmp"; then
            echo "::warning::could not fetch $repo/$file from main — if this persists, the file was renamed or replaced upstream"
        elif [ "$(sha256 "$tmp")" = "$sum" ]; then
            echo "$file is current"
        else
            echo "::warning::$file changed upstream — move its pin in scripts/references.sh and refresh tests/fixtures/"
        fi
    done < <(pins)
    exit 0
fi

mkdir -p "$dest"
fail=0
while read -r repo commit file sum; do
    out="$dest/$file"
    if [ ! -f "$out" ] || [ "$(sha256 "$out")" != "$sum" ]; then
        curl -sSfL --retry 3 "$base/$repo/$commit/$file" -o "$out.part" && mv "$out.part" "$out" || {
            echo "✗ could not fetch $repo/$file at $commit" >&2; fail=1; continue; }
    fi
    if [ "$(sha256 "$out")" = "$sum" ]; then
        echo "✓ $file"
    else
        echo "✗ $file does not match its pinned SHA-256" >&2; fail=1
    fi
done < <(pins)
[ "$fail" -eq 0 ] || exit 1
echo "$(pins | wc -l | tr -d ' ') documents in $dest"
