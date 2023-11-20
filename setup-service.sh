SERVICE_PATH="/etc/systemd/system/blog_backend.service"
EXECUTABLE_PATH="/root/blog_backend/target/debug/blog_backend"

if [! [ -e $SERVICE_PATH ]]; then
    touch $SERVICE_PATH
fi

read -p 'DATABASE_URL: ' database_url
read -p 'JWT_SIGNING_KEY: ' jwt_signing_key

cat >$SERVICE_PATH <<-EOM
[Unit]
Description=uhqblog backend service
After=network.target
StartLimitIntervalSec=0
RestartSec=5
[Service]
Type=simple
Restart=always
RestartSec=1
User=root
ExecStart=$EXECUTABLE_PATH
Environment=DATABASE_URL=$database_url
Environment=JWT_SIGNING_KEY=$jwt_signing_key

[Install]
WantedBy=multi-user.target
EOM

echo "已存储服务到 $SERVICE_PATH"
