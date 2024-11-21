#!/usr/bin/env bash

# ------------- Constants
# Default .tms directory path
tms_install_dir=${HOME}/.tms
echo 'tms installation directory: ' ${tms_install_dir}

# Directory for local installation customizations.
tms_customizations_dir=${HOME}/tms_customizations
echo 'tms local customizations directory: ' ${tms_customizations_dir} 

# TMS code repository directory.
tms_code_dir=${HOME}/tms_server
echo 'tms code directory: ' ${tms_code_dir}

#set -x

# ---------------------------------------------------
# Start Processing
# ---------------------------------------------------
# ------------- Validation
# Make sure rust is installed.
rustc --version
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to access rustc. Install the latest stable version of Rust if necessary.'
    exit 10
fi

# Create the local customization  directory if it doesn't exist.
if ! [[ -d ${tms_customizations_dir} ]]; then
    mkdir ${tms_customizations_dir}
    if [[ $? != 0 ]]; then
   	echo 'ERROR: Unable to create "'  ${tms_customizations_dir} '" directory.'
    	return 20
    fi
    chmod 700 ${tms_customizations_dir}    
fi

# Copy the example certificate path files to the customization directory.
if ! [[ -r ${tms_customizations_dir}/cert.path ]]; then
    cp -p ${tms_code_dir}/deployment/native/cert.path ${tms_customizations_dir}
    if [[ $? != 0 ]]; then
        echo 'ERROR: Unable to create "'  ${tms_customizations_dir}/cert.path '.'
        return 22
    fi
fi
if ! [[ -r ${tms_customizations_dir}/key.path ]]; then
    cp -p ${tms_code_dir}/deployment/native/key.path ${tms_customizations_dir}
    if [[ $? != 0 ]]; then
        echo 'ERROR: Unable to create "'  ${tms_customizations_dir}/key.path '.'
        return 24
    fi
fi

# ------------- Begin Build
# Move to the top-level directory of the tms_server codebase.
cd ${tms_code_dir}
if [[ $? != 0 ]]; then
    echo 'ERROR: Could not find' "${tms_code_dir}" 'directory.'
    exit 30    
fi

# Build tms_server and all its dependencies.
cargo build --release
if [[ $? != 0 ]]; then
    echo 'ERROR: Release build failed!'
    exit 40
fi

# Copy optimized executable to the /opt/tms directory.
cp -p target/release/tms_server /opt/tms_server
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to copy target/release/tms_server to /opt/tms_server/tms_server.'
    exit 50
fi
chmod 770 /opt/tms_server/tms_server
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to chmod on /opt/tms_server/tms_server.'
    exit 51
fi

# Make sure the systemd directory subtree exists.
mkdir -p /opt/tms_server/lib/systemd/system
if [[ $? != 0 ]]; then
    echo 'ERROR: Unable to create /opt/tms_server/lib/systemd/system directory.'
    exit 53
fi

# Copy the systemd unit file if it doesn't exist.
if ! [[ -r /opt/tms_server/lib/systemd/system/tms_server.service ]]; then
    cp -p deployment/native/tms_server.service /opt/tms_server/lib/systemd/system
    if [[ $? != 0 ]]; then
        echo 'ERROR: Unable to copy deployment/native/tms_server.service to /opt/tms_server/lib/systemd/system.'
        exit 55
    fi
fi

# ------------- First Time Install Processing
if ! [[ -d ${tms_install_dir} ]]; then
    # Initialize the content of the install directory.
    /opt/tms_server/tms_server --install > ${tms_customizations_dir}/tms-install.out 2>&1
    if [[ $? != 0 ]]; then
       echo 'ERROR: Aborting due to tms_server first-time installation failure.'
       rm -fr ${tms_install_dir}
       exit 60
    fi
    chmod 660 ${tms_customizations_dir}/tms-install.out
    if [[ $? != 0 ]]; then
       echo 'ERROR: Aborting due to chmod failure on ' ${tms_customizations_dir}/tms-install.out '.'
       rm -fr ${tms_install_dir}
       exit 62
    fi   
fi	

# ------------- Copy Local Customizations
# Copy local tms configuration file.
if [[ -r ${tms_customizations_dir}/tms.toml ]]; then
    # Copy the custom tms configuration file to the .tms config directory.
    cp -p "${tms_customizations_dir}/tms.toml" "${tms_install_dir}/config"
    if [[ $? != 0 ]]; then
       echo 'ERROR: Unable to copy' "${tms_customizations_dir}/tms.toml" "to ${tms_install_dir}/config"
       rm -fr ${tms_install_dir}
       exit 70
    fi    
fi	

# Copy local log configuration file.
if [[ -r ${tms_customizations_dir}/log4rs.yml ]]; then
    # Copy the custom tms configuration file to the .tms config directory.
    cp -p "${tms_customizations_dir}/log4rs.yml" "${tms_install_dir}/config"
    if [[ $? != 0 ]]; then
       echo 'ERROR: Unable to copy' "${tms_customizations_dir}/log4rs.yml" "to ${tms_install_dir}/config"
       rm -fr ${tms_install_dir}
       exit 80
    fi
fi

echo "**** tms_server successfully installed and running ****"
