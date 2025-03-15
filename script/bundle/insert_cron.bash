#! /bin/bash
# insert_cron.sh - Insert the cron job, if it does not exist.

cron_job="5 * * * * /bin/bash /etc/regis/daemon_check.sh"

(crontab -l 2>/dev/null | grep -v -F "$cron_job"; echo "$cron_job") | crontab -