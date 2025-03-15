#! /bin/bash
# daemon_check.bash
# Determine if the regisd service is running.

if [ ! /usr/bin/regisc poll ]; then 
    if [ ! systemctl restart regisd ]; then
        echo Unable to restart service. Ensure that this script is run with root ability.
        exit 1
    fi 
fi
