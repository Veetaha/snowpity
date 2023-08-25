set -euo pipefail

# I wish the world was simpler, and used a single convention for naming architectures
# but it doesn't. So we need to use different arch names for different tools.
# Produces Go-style arch names, e.g. amd64, arm64, etc.
export arch_go=$(dpkg --print-architecture)

# Produces Rust-style arch names, e.g. x86_64, aarch64, etc.
export arch_rust=$(uname -m)

function curl_and_decompress {
    local url="$1"
    shift

    if [[ $url == *.tar.gz || $url == *.tgz ]]
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
    shift

    curl \
        --retry 5 \
        --retry-connrefused \
        --retry-delay 30 \
        -L "$url" "$@"
}
