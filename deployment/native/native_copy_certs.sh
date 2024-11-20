#!/usr/bin/env bash

# ------------- Constants
# Default .tms directory path is hardcoded.
tms_install_dir=/home/tms/.tms
echo 'tms installation directory: ' ${tms_install_dir}

# Local customization directory is hardcoded.
tms_customizations=/home/tms/tms_customizations
echo 'tms local customizations directory:' ${tms_customizations}
#set -x

# ------------- Validation
# Make sure the cert environment variable is set.
if ! [[ -r ${tms_customizations}/cert.path ]]; then
    echo "ERROR: The required cert.path file must be readable in the ${tms_customizations} "
    echo "directory and contain the path to the certificate chain file." 
    exit 1
fi

# Make sure we have access to the host's private key.
if ! [[ -r ${tms_customizations}/key.path ]]; then
    echo "ERROR: The required key.path file must be readable in the ${tms_customizations} "
    echo "directory and contain the path to the host's private key file." 
    exit 1
fi

# ------------- Read Certificate and Key Paths
cert_path=$(cat ${tms_customizations}/cert.path)
key_path=$(cat ${tms_customizations}/key.path)

# ------------- Execution
# Copy certs file from host directory to installation directory.
cp -p "${cert_path}" "${tms_install_dir}/certs/cert.pem"
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to copy' ${cert_path} "to ${tms_install_dir}/certs/cert.pem"
    exit 1
fi

# Copy key file from host directory to installation directory.
cp -p "${key_path}" "${tms_install_dir}/certs/key.pem"
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to copy' "${key_path}" "to ${tms_install_dir}/certs/key.pem"
    rm "${tms_install_dir}/certs/cert.pem"
    exit 1
fi

# Change permissions and ownership.
chmod 600 "${tms_install_dir}/certs/cert.pem" ${tms_install_dir}/certs/key.pem
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to chmod on' "${tms_install_dir}/certs/cert.pem" "and ${tms_install_dir}/certs/key.pem"
    rm "${tms_install_dir}/certs/cert.pem" "${tms_install_dir}/certs/key.pem"
    exit 1
fi

chown tms:tms "${tms_install_dir}/certs/cert.pem" "${tms_install_dir}/certs/key.pem"
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to chown on' "${tms_install_dir}/certs/cert.pem" "and ${tms_install_dir}/certs/key.pem"
    rm "${tms_install_dir}/certs/cert.pem" "${tms_install_dir}/certs/key.pem"
    exit 1
fi

