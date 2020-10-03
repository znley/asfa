#!/usr/bin/env bash

set -euo pipefail

sourcedir="$(dirname "$(readlink -m "${BASH_SOURCE[0]}")")"

####################
# Helper functions #
####################

export_var() {
    local name="$1"
    local value="$2"

    eval "export ${name}=\"${value}\""
    echo "export ${name}=\"${value}\""
}

ensure_folder() {
    local folder="${1}"
    if [ ! -d "${folder}" ]; then
        mkdir -p "${folder}"
    fi
}

##########################
# Setup test root folder #
##########################

testroot="$(mktemp -d)"
ensure_folder "${testroot}"

###############
# Copy config #
###############

ensure_folder "${testroot}"
export_var ASFA_TEST_ROOT "${testroot}"

folder_config="${testroot}/config"
ensure_folder "${folder_config}"

cp "${sourcedir}/ci-config/raw.yaml" "${folder_config}/config.yaml"
export_var ASFA_CONFIG "${folder_config}"

#########################
# Restore test-ssh keys #
#########################

TEST_SSH_PRIVKEY_FILE="${ASFA_CONFIG}/test.key"

echo -n "${TEST_SSH_PRIVKEY_B64}" | openssl base64 -A -d > "${TEST_SSH_PRIVKEY_FILE}"
sed -i "s:TEST_SSH_PRIVKEY_FILE:${TEST_SSH_PRIVKEY_FILE}:" "${ASFA_CONFIG}/config.yaml"

TEST_SSH_PUBKEY_FILE="${testroot}/test.pub"
echo "${TEST_SSH_PUBKEY}" > "${TEST_SSH_PUBKEY_FILE}"

#################
# Set up docker #
#################

export_var ASFA_FOLDER_UPLOAD "${testroot}/uploads"
ensure_folder "${ASFA_FOLDER_UPLOAD}"

folder_docker_config="${testroot}/docker-cfg"
ensure_folder "${folder_docker_config}"

if (( $(docker container ls -q | wc -l) == 0 )); then
    docker build -t asfa-ci-image - >&2 <<EOF
FROM linuxserver/openssh-server
# needed for scp-functionality
RUN apk add --no-cache openssh-client
EOF
    docker create \
      --name=asfa-ci \
      -e "PUID=$(id -u)" \
      -e "PGID=$(id -g)" \
      -e TZ=Europe/London \
      -e PUBLIC_KEY_FILE=test.pub \
      -e PASSWORD_ACCESS=true \
      -e USER_PASSWORD=foobar \
      -e USER_NAME=asfa-ci-user \
      -p 2222:2222 \
      -v "${ASFA_FOLDER_UPLOAD}:/var/www/default/uploads" \
      -v "${folder_docker_config}":/config \
      --restart unless-stopped \
      asfa-ci-image >&2

    docker start asfa-ci >&2
fi
