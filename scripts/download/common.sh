set -euo pipefail

function curl_tar_gz {
    local url="$1"
    shift

    curl \
    --retry 5 \
    --retry-connrefused \
    --retry-delay 30 \
    -L "$url" \
    | tar --gzip --extract "$@"
}
