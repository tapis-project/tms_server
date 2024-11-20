#!/usr/bin/env bash

# ------------- Constants
# Default .tms directory path is hardcoded.
tms_install_dir=/home/tms/.tms
echo 'tms installation directory: ' ${tms_install_dir}

# ------------- Validation
# Make sure the cert environment variable is set.
if [[ -z ${TMS_CERT_FILE} ]]; then
    echo "ERROR: The required environment variable TMS_CERT_FILE must be set "
    echo "to the non-empty path of the host's full certificate chain file." 
    exit 1
fi

# Make sure we have access to the host's private key.
if [[ -z ${TMS_PRIV_KEY_FILE} ]]; then
    echo "ERROR: The required environment variable TMS_PRIV_KEY_FILE "
    echo "must be set to non-empty path of the host's private key file." 
    exit 1
fi

# ------------- Execution
# Copy certs file from host directory to installation directory.
cp -p "${TMS_CERT_FILE}" "${tms_install_dir}/certs/cert.pem"
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to copy' ${TMS_CERT_FILE} "to ${tms_install_dir}/certs/cert.pem"
    exit 1
fi

# Copy key file from host directory to installation directory.
cp -p "${TMS_PRIV_KEY_FILE}" "${tms_install_dir}/certs/key.pem"
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to copy' "${TMS_PRIV_KEY_FILE}" "to ${tms_install_dir}/certs/key.pem"
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


