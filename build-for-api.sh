#!/bin/bash

# Script para build e preparação para upload na API Percolist
# Uso: ./build-for-api.sh <version> <release_notes>

if [ $# -lt 2 ]; then
    echo "Uso: $0 <version> <release_notes>"
    echo "Exemplo: $0 1.0.2 'Correções de bugs e melhorias'"
    exit 1
fi

VERSION=$1
RELEASE_NOTES=$2

echo "🚀 Iniciando build para API Percolist - versão $VERSION"

# 1. Atualizar versões nos arquivos
echo "📝 Atualizando versões..."
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" src-tauri/Cargo.toml
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json
sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" package.json

# 2. Build do projeto
echo "🔨 Fazendo build..."
export TAURI_SIGNING_PRIVATE_KEY="~/.tauri/percolist.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="sua_senha_aqui"

npm run tauri build

# 3. Gerar assinaturas
echo "🔐 Gerando assinaturas..."
cd src-tauri/target/release/bundle

# Para Windows
if [ -f "nsis/Percolist_Panel_${VERSION}_x64-setup.exe" ]; then
    echo "📝 Assinando Windows..."
    tauri signer sign "nsis/Percolist_Panel_${VERSION}_x64-setup.exe" > "nsis/Percolist_Panel_${VERSION}_x64-setup.exe.sig"
    WINDOWS_SIGNATURE=$(cat "nsis/Percolist_Panel_${VERSION}_x64-setup.exe.sig")
fi

# Para Linux
if [ -f "appimage/Percolist_Panel_${VERSION}_amd64.AppImage" ]; then
    echo "📝 Assinando Linux..."
    tauri signer sign "appimage/Percolist_Panel_${VERSION}_amd64.AppImage" > "appimage/Percolist_Panel_${VERSION}_amd64.AppImage.sig"
    LINUX_SIGNATURE=$(cat "appimage/Percolist_Panel_${VERSION}_amd64.AppImage.sig")
fi

# Para macOS
if [ -f "dmg/Percolist_Panel_${VERSION}_x64.dmg" ]; then
    echo "📝 Assinando macOS Intel..."
    tauri signer sign "dmg/Percolist_Panel_${VERSION}_x64.dmg" > "dmg/Percolist_Panel_${VERSION}_x64.dmg.sig"
    MACOS_INTEL_SIGNATURE=$(cat "dmg/Percolist_Panel_${VERSION}_x64.dmg.sig")
fi

if [ -f "dmg/Percolist_Panel_${VERSION}_arm64.dmg" ]; then
    echo "📝 Assinando macOS Apple Silicon..."
    tauri signer sign "dmg/Percolist_Panel_${VERSION}_arm64.dmg" > "dmg/Percolist_Panel_${VERSION}_arm64.dmg.sig"
    MACOS_ARM_SIGNATURE=$(cat "dmg/Percolist_Panel_${VERSION}_arm64.dmg.sig")
fi

cd ../../..

# 4. Gerar arquivos JSON para cada plataforma
echo "📄 Gerando arquivos JSON para API..."

# Windows
cat > api-files/windows-x86_64.json << EOF
{
  "version": "v$VERSION",
  "notes": "$RELEASE_NOTES",
  "pub_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platforms": {
    "windows-x86_64": {
      "signature": "$WINDOWS_SIGNATURE",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_${VERSION}_x64-setup.exe"
    }
  }
}
EOF

# Linux
cat > api-files/linux-x86_64.json << EOF
{
  "version": "v$VERSION",
  "notes": "$RELEASE_NOTES",
  "pub_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platforms": {
    "linux-x86_64": {
      "signature": "$LINUX_SIGNATURE",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_${VERSION}_amd64.AppImage"
    }
  }
}
EOF

# macOS Intel
cat > api-files/darwin-x86_64.json << EOF
{
  "version": "v$VERSION",
  "notes": "$RELEASE_NOTES",
  "pub_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platforms": {
    "darwin-x86_64": {
      "signature": "$MACOS_INTEL_SIGNATURE",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_${VERSION}_x64.dmg"
    }
  }
}
EOF

# macOS Apple Silicon
cat > api-files/darwin-aarch64.json << EOF
{
  "version": "v$VERSION",
  "notes": "$RELEASE_NOTES",
  "pub_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "platforms": {
    "darwin-aarch64": {
      "signature": "$MACOS_ARM_SIGNATURE",
      "url": "https://api.percolist.com.br/downloads/Percolist_Panel_${VERSION}_arm64.dmg"
    }
  }
}
EOF

echo "✅ Build e preparação concluídos!"
echo ""
echo "📋 Arquivos gerados:"
echo "📁 src-tauri/target/release/bundle/ - Binários assinados"
echo "📁 api-files/ - Arquivos JSON para sua API"
echo ""
echo "📤 Próximos passos:"
echo "1. Faça upload dos binários para: https://api.percolist.com.br/downloads/"
echo "2. Configure os endpoints da sua API com os arquivos de api-files/"
echo "3. Teste o app: npm run tauri dev"
echo ""
echo "🔗 Endpoints da API:"
echo "Windows: https://api.percolist.com.br/updates/windows/x86_64/latest.json"
echo "Linux: https://api.percolist.com.br/updates/linux/x86_64/latest.json"
echo "macOS Intel: https://api.percolist.com.br/updates/darwin/x86_64/latest.json"
echo "macOS Apple Silicon: https://api.percolist.com.br/updates/darwin/aarch64/latest.json"
