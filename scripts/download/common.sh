set -euo pipefail

function curl_and_decompress {
    local url="$1"
    shift

    if [[ $url == *.tar.gz ]]
    then
        curl_with_retry $url | tar --extract --gzip "$@"
    elif [[ $url == *.tar.xz ]]
    then
        curl_with_retry $url | tar --extract --xz "$@"
    elif [[ $url == *.gz ]]
    then
        curl_with_retry $url | gzip --decompress > $(basename $url .gz)
    else
        echo "Unknown file type: $url"
        exit 1
    fi
}

function curl_with_retry {
    local url="$1"

    curl \
        --retry 5 \
        --retry-connrefused \
        --retry-delay 30 \
        -L "$url"
}
