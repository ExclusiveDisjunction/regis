#! /bin/bash
# create_user.bash - Create the user for the regisd service.

username="regisd_user"
groupname="regisd_group"

if ! id "$username" &>/dev/null; then 
    useradd --system --no-create-home --shell /usr/sbin/nologin regisd_user
fi 

if ! getent group "$groupname" &>/dev/null; then 
    groupadd regisd_group
    usermod -a -G regisd_group regisd_user
fi

chown -R regisd_user:regisd_group /usr/bin/regisd
chown -R regisd_user:regisd_group /etc/regis