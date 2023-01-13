set -euo pipefail

function curl_and_decompress {
    local url="$1"
    shift

    if [[ $url == *.tar.gz ]]; then
        curl \
        --retry 5 \
        --retry-connrefused \
        --retry-delay 30 \
        -L "$url" \
        | tar --gzip --extract "$@"
    elif [[ $url == *.gz ]]; then
        curl \
        --retry 5 \
        --retry-connrefused \
        --retry-delay 30 \
        -L "$url" \
        | gzip --decompress > $(basename $url .gz)
    else
        echo "Unknown file type: $url"
        exit 1
    fi
}
