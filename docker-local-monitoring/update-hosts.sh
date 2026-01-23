#!/bin/bash

# Add entries to /etc/hosts for local monitoring services
# Run with: sudo ./update-hosts.sh

HOSTS_ENTRIES="
127.0.0.1 grafana.myhome.com
127.0.0.1 prometheus.myhome.com
127.0.0.1 pgadmin.myhome.com
127.0.0.1 postgres-exporter.myhome.com
127.0.0.1 node-exporter.myhome.com
"

echo "Adding entries to /etc/hosts..."
echo "$HOSTS_ENTRIES" | sudo tee -a /etc/hosts > /dev/null
echo "Done! You can now access your services at:"
echo "  - https://grafana.myhome.com"
echo "  - https://prometheus.myhome.com"
echo "  - https://pgadmin.myhome.com"
echo "  - https://postgres-exporter.myhome.com"
echo "  - https://node-exporter.myhome.com"
