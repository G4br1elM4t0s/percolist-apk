#!/bin/bash

echo "🔐 Gerando assinatura válida..."
echo "================================="

# Configurar variáveis
export TAURI_SIGNING_PRIVATE_KEY="/c/Users/Gabriel/.tauri/keys"

# Arquivo para assinar
FILE="src-tauri/target/release/bundle/nsis/Percolist_Panel_1.0.0_x64-setup.exe"

if [ ! -f "$FILE" ]; then
    echo "❌ Arquivo não encontrado: $FILE"
    echo "Execute primeiro: npm run tauri build"
    exit 1
fi

echo "📝 Assinando arquivo: $FILE"

# Gerar assinatura
npm run tauri signer sign "$FILE"

# Verificar se foi gerada
SIG_FILE="$FILE.sig"
if [ -f "$SIG_FILE" ]; then
    echo "✅ Assinatura gerada com sucesso!"
    echo "📄 Conteúdo da assinatura:"
    cat "$SIG_FILE"
    echo ""
    echo "🔗 Para usar na sua API, copie o conteúdo acima"
else
    echo "❌ Erro ao gerar assinatura"
    exit 1
fi
