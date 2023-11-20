#!/bin/bash

SERVICE_PATH="/etc/systemd/system/blog_backend.service"
SERVICE_DIR_PATH="$SERVICE_PATH.d"
ENV_FILE_PATH="$SERVICE_DIR_PATH/myenv.conf"
EXECUTABLE_PATH="/root/backend/blog_backend"

mkdir -p $SERVICE_DIR_PATH

if [ ! -e $SERVICE_PATH ]; then
    touch $SERVICE_PATH
fi

if [ ! -e $ENV_FILE_PATH ]; then
    touch $ENV_FILE_PATH
fi

read -e -p 'DATABASE_URL: ' database_url
read -e -p 'JWT_SIGNING_KEY: ' jwt_signing_key

cat >$SERVICE_PATH <<-EOM
[Unit]
Description=uhqblog backend service
After=network.target
StartLimitIntervalSec=0
[Service]
Type=simple
Restart=always
RestartSec=10
User=root
ExecStart=$EXECUTABLE_PATH

[Install]
WantedBy=multi-user.target
EOM

cat >$ENV_FILE_PATH <<-EOM
[Service]
Environment="DATABASE_URL=${database_url//\%/\%\%}"
Environment="JWT_SIGNING_KEY=${jwt_signing_key//\%/\%\%}"
EOM

echo "已存储服务到 $SERVICE_PATH"
