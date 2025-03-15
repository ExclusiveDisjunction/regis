#! /bin/bash
# remove_cron.sh - remove the cron tab entry.

cron_job="5 * * * * /bin/bash /etc/regis/daemon_check.sh"

(crontab -l 2>/dev/null | grep -v -F "$cron_job") | crontab -