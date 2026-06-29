#!/bin/bash
echo "Instalando dependencias del sistema operativo para GIA..."
sudo apt update
wget https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6.1-2/wkhtmltox_0.12.6.1-2.jammy_amd64.deb
sudo apt install -y ./wkhtmltox_0.12.6.1-2.jammy_amd64.deb
rm wkhtmltox_0.12.6.1-2.jammy_amd64.deb
echo "¡Dependencias instaladas! Ya podés ejecutar 'cargo run'."