#!/bin/bash

echo "🔐 Configurando chaves de assinatura..."
echo "========================================"

# Configurar variáveis de ambiente com as chaves corretas
export TAURI_SIGNING_PRIVATE_KEY="C:/Users/Gabriel/.tauri/keys"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="sua_senha_aqui"

echo "✅ Chaves configuradas:"
echo "🔑 Chave Privada: C:/Users/Gabriel/.tauri/keys"
echo "🔑 Chave Pública: dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDZEN0NEMEZBMTUxODE0OTkKUldTWkZCZ1YrdEI4YmVZMythNTFQVnF2NHEvZHNZVGo4QTRQa1hRdDdXZlVoQ0RFb084YTQrTDQK"
echo ""
echo "📋 Para usar:"
echo "source setup-keys.sh"
echo "npm run tauri build"
echo ""
echo "🔧 Para testar assinatura:"
echo "npm run tauri signer sign arquivo.exe"
