#!/bin/bash

echo "🔧 Configurando Sistema de Auto-Update do Percolist"
echo "=================================================="

# 1. Verificar se as chaves existem
if [ ! -f ~/.tauri/percolist.key ]; then
    echo "❌ Chave privada não encontrada!"
    echo "Execute: npm run tauri signer generate -- -w ~/.tauri/percolist.key"
    exit 1
fi

echo "✅ Chave privada encontrada"

# 2. Perguntar informações do GitHub
echo ""
echo "📋 Configuração do GitHub"
echo "-------------------------"
read -p "Digite seu nome de usuário do GitHub: " GITHUB_USER
read -p "Digite o nome do repositório: " GITHUB_REPO
read -p "Digite a senha da chave privada: " PRIVATE_KEY_PASSWORD

# 3. Atualizar tauri.conf.json
echo ""
echo "🔧 Atualizando configuração..."
sed -i "s|https://raw.githubusercontent.com/seu-usuario/seu-repo/main/latest.json|https://raw.githubusercontent.com/$GITHUB_USER/$GITHUB_REPO/main/latest.json|g" src-tauri/tauri.conf.json

echo "✅ URL do GitHub atualizada"

# 4. Criar arquivo de configuração de ambiente
echo ""
echo "🔐 Criando arquivo de configuração..."
cat > .env.update << EOF
# Configurações para Auto-Update
TAURI_SIGNING_PRIVATE_KEY=~/.tauri/percolist.key
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=$PRIVATE_KEY_PASSWORD
GITHUB_USER=$GITHUB_USER
GITHUB_REPO=$GITHUB_REPO
EOF

echo "✅ Arquivo .env.update criado"

# 5. Atualizar latest.json com as informações corretas
echo ""
echo "📄 Atualizando latest.json..."
cat > latest.json << EOF
{
  "version": "v1.0.1",
  "notes": "Versão inicial com auto-update",
  "pub_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platforms": {
    "darwin-x86_64": {
      "signature": "Content of Percolist_Panel_1.0.1_x64.dmg.sig",
      "url": "https://github.com/$GITHUB_USER/$GITHUB_REPO/releases/download/v1.0.1/Percolist_Panel_1.0.1_x64.dmg"
    },
    "darwin-aarch64": {
      "signature": "Content of Percolist_Panel_1.0.1_arm64.dmg.sig",
      "url": "https://github.com/$GITHUB_USER/$GITHUB_REPO/releases/download/v1.0.1/Percolist_Panel_1.0.1_arm64.dmg"
    },
    "linux-x86_64": {
      "signature": "Content of Percolist_Panel_1.0.1_amd64.AppImage.sig",
      "url": "https://github.com/$GITHUB_USER/$GITHUB_REPO/releases/download/v1.0.1/Percolist_Panel_1.0.1_amd64.AppImage"
    },
    "windows-x86_64": {
      "signature": "Content of Percolist_Panel_1.0.1_x64-setup.exe.sig",
      "url": "https://github.com/$GITHUB_USER/$GITHUB_REPO/releases/download/v1.0.1/Percolist_Panel_1.0.1_x64-setup.exe"
    }
  }
}
EOF

echo "✅ latest.json atualizado"

# 6. Instruções finais
echo ""
echo "🎉 Configuração concluída!"
echo ""
echo "📋 Próximos passos:"
echo "1. Faça upload do latest.json para: https://github.com/$GITHUB_USER/$GITHUB_REPO"
echo "2. Execute: source .env.update"
echo "3. Execute: npm run tauri build"
echo "4. Crie um release no GitHub com os arquivos de src-tauri/target/release/bundle/"
echo "5. Teste o app: npm run tauri dev"
echo ""
echo "🔧 Para fazer releases futuros:"
echo "./build-and-release.sh 1.0.2 'Correções de bugs'"
