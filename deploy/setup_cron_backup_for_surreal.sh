#!/bin/bash
set -euo pipefail

# ------------------------------------------------------------------------------
# What you should do on the server
# ------------------------------------------------------------------------------
# 1. Install the SurrealDB CLI on the host if needed (e.g. from surrealdb.com/install),
#    or set SURREAL_BIN below to wherever you install it.
# 2. Create the password file and restrict it:
#    - Path: same as SURREAL_PASSWORD_FILE below (e.g. /home/thebogie/stg/stg_prod/.password.surreal)
#    - Content: the production SurrealDB root password (e.g. from deploy/config/.env.prod SURREAL_PASSWORD).
#    - chmod 600 (this script also runs chmod 600 when it checks the file).
# 3. Run this setup script once as root:
#    sudo ./setup_cron_backup_for_surreal.sh
#    (from the deploy/ directory after scp deploy/ to the server)
# 4. Optionally run the cron script once by hand to confirm backups and log:
#    sudo /etc/cron.hourly/surrealdb_backup
#    Then check /mnt/homelab_backup/stg/backups/surrealdb_backup.log and that timestamped
#    dirs appear under /mnt/homelab_backup/stg/backups/.
# ------------------------------------------------------------------------------

# === Ensure Script is Run as Root ===
if [[ $EUID -ne 0 ]]; then
  echo "❌ Please run this script as root."
  exit 1
fi

# === Config (single source of truth; used by both this script and the generated cron job) ===
BACKUP_DIR="/mnt/homelab_backup/stg/backups"
CRON_JOB_FILE="/etc/cron.hourly/surrealdb_backup"
SURREAL_BIN="/usr/local/bin/surreal"  # Update if your binary is elsewhere (e.g. /usr/bin/surreal)
SURREAL_ENDPOINT="http://127.0.0.1:50001"  # Host port; must match SURREALDB_PORT in deploy (container 8000 → 50001)
SURREAL_USER="root"
SURREAL_PASSWORD_FILE="/home/thebogie/stg/stg_prod/.password.surreal"  # Create with prod root password; chmod 600
LOG_FILE="$BACKUP_DIR/surrealdb_backup.log"
RETENTION_DAYS=7

# SurrealDB namespace and databases (must match deploy/config/.env.prod: SURREAL_NS, SURREAL_DB)
SURREAL_NS="stg_rd"
DATABASES=("system" "stg_rd")

# === Ensure Backup Directory Exists (needed before any tee to LOG_FILE) ===
mkdir -p "$BACKUP_DIR"
if [ ! -w "$BACKUP_DIR" ]; then
  echo "❌ Backup directory $BACKUP_DIR is not writable."
  exit 1
fi

# === Check SurrealDB Binary ===
if [ ! -x "$SURREAL_BIN" ]; then
  echo "❌ SurrealDB binary not found or not executable at $SURREAL_BIN"
  exit 1
fi

# === Check Password File ===
if [ ! -f "$SURREAL_PASSWORD_FILE" ]; then
  echo "❌ Password file $SURREAL_PASSWORD_FILE not found. Create it with the SurrealDB root password and chmod 600."
  exit 1
fi
chmod 600 "$SURREAL_PASSWORD_FILE"

# === Create Backup Cron Script (variables from Config above are expanded into the file) ===
tee "$CRON_JOB_FILE" > /dev/null <<EOF
#!/bin/bash
set -euo pipefail

TIMESTAMP=\$(date +"%Y-%m-%d_%H-%M-%S")
DEST="$BACKUP_DIR/\$TIMESTAMP"
LOG_FILE="$LOG_FILE"
SURREAL_BIN="$SURREAL_BIN"
SURREAL_PASSWORD_FILE="$SURREAL_PASSWORD_FILE"
SURREAL_ENDPOINT="$SURREAL_ENDPOINT"
SURREAL_NS="$SURREAL_NS"
DATABASES=($(printf '"%s" ' "${DATABASES[@]}"))
RETENTION_DAYS=$RETENTION_DAYS

export SURREAL_PASSWORD=\$(< "\$SURREAL_PASSWORD_FILE")

mkdir -p "\$DEST" || { echo "\$TIMESTAMP: ❌ Failed to create backup directory \$DEST" >> "\$LOG_FILE"; exit 1; }

for DB in "\${DATABASES[@]}"; do
  EXPORT_FILE="\$DEST/\${DB}.surql"

  "\$SURREAL_BIN" export \\
    --endpoint "\$SURREAL_ENDPOINT" \\
    --user $SURREAL_USER \\
    --pass "\$SURREAL_PASSWORD" \\
    --ns "\$SURREAL_NS" \\
    --db "\$DB" \\
    "\$EXPORT_FILE" >> "\$LOG_FILE" 2>&1

  if [ \$? -eq 0 ]; then
    gzip "\$EXPORT_FILE"
    echo "\$TIMESTAMP: ✅ Backup of database '\$DB' successful to \${EXPORT_FILE}.gz" >> "\$LOG_FILE"
  else
    echo "\$TIMESTAMP: ❌ Backup of database '\$DB' failed." >> "\$LOG_FILE"
    exit 1
  fi
done

# Cleanup old backups
OLD_BACKUPS=\$(find "$BACKUP_DIR" -mindepth 1 -maxdepth 1 -type d -mtime +\$RETENTION_DAYS)
if [ -n "\$OLD_BACKUPS" ]; then
  echo "\$TIMESTAMP: 🧹 Removing backups older than \$RETENTION_DAYS days" >> "\$LOG_FILE"
  echo "\$OLD_BACKUPS" | xargs rm -rf
fi
EOF

# === Make Cron Script Executable ===
chmod +x "$CRON_JOB_FILE"

# === Ensure Log File Exists and Has Correct Permissions ===
touch "$LOG_FILE"
chmod 644 "$LOG_FILE"

echo "✅ SurrealDB hourly backup job created successfully."
