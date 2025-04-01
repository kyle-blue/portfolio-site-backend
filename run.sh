#/bin/bash

echo Script for running in dev mode only...
echo Please input your email password:

read EMAIL_PASSWORD

ENVIRONMENT=dev EMAIL_ADDRESS=kyle.blue.doidge@gmail.com ALLOWED_ORIGINS=* EMAIL_PASSWORD=$EMAIL_PASSWORD cargo run